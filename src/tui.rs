//! Interactive viewer: segment list, decoded field table and live findings.

use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::parser::Message;
use crate::render;
use crate::spec;
use crate::text::{fit, truncate};
use crate::validate::{Report, Severity};
use crate::view::{segment_rows, FieldRow, RowOptions};

const ACCENT: Color = Color::Cyan;
const MUTED: Color = Color::DarkGray;

fn severity_style(sev: Option<Severity>) -> Style {
    match sev {
        None => Style::default().fg(Color::Green),
        Some(Severity::Error) => Style::default().fg(Color::Red),
        Some(Severity::Warning) => Style::default().fg(Color::Yellow),
        Some(Severity::Info) => Style::default().fg(MUTED),
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Focus {
    Segments,
    Fields,
    Findings,
}

struct App {
    items: Vec<(String, Message, Report)>,
    current: usize,
    segments: ListState,
    fields: ListState,
    findings: ListState,
    focus: Focus,
    show_empty: bool,
    verbose: bool,
    show_raw: bool,
    all_findings: bool,
    help: bool,
}

impl App {
    fn new(items: Vec<(String, Message, Report)>) -> Self {
        let mut segments = ListState::default();
        segments.select(Some(0));
        Self {
            items,
            current: 0,
            segments,
            fields: ListState::default(),
            findings: ListState::default(),
            focus: Focus::Segments,
            show_empty: false,
            verbose: false,
            show_raw: false,
            all_findings: true,
            help: false,
        }
    }

    fn msg(&self) -> &Message {
        &self.items[self.current].1
    }
    fn report(&self) -> &Report {
        &self.items[self.current].2
    }
    fn label(&self) -> &str {
        &self.items[self.current].0
    }
    fn seg_index(&self) -> usize {
        self.segments
            .selected()
            .unwrap_or(0)
            .min(self.msg().segments.len() - 1)
    }

    fn move_message(&mut self, delta: isize) {
        // Wrap around the list without signed index arithmetic: reducing
        // `delta` modulo the length first keeps the addition non-negative.
        let len = self.items.len();
        if len == 0 {
            return;
        }
        let Ok(len_signed) = isize::try_from(len) else {
            return;
        };
        let step = usize::try_from(delta.rem_euclid(len_signed)).unwrap_or(0);
        let next = (self.current + step) % len;
        if next != self.current {
            self.current = next;
            self.segments.select(Some(0));
            self.fields.select(None);
            self.findings.select(None);
        }
    }

    fn step(&mut self, delta: isize) {
        let len = match self.focus {
            Focus::Segments => self.items[self.current].1.segments.len(),
            Focus::Fields => self.field_len(),
            Focus::Findings => self.finding_len(),
        };
        let state = match self.focus {
            Focus::Segments => &mut self.segments,
            Focus::Fields => &mut self.fields,
            Focus::Findings => &mut self.findings,
        };
        if len == 0 {
            return;
        }
        let current = state.selected().unwrap_or(0);
        let next = if delta < 0 {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current.saturating_add(delta.unsigned_abs()).min(len - 1)
        };
        state.select(Some(next));
        if self.focus == Focus::Segments {
            self.fields.select(None);
        }
    }

    fn field_len(&self) -> usize {
        self.rows().len()
    }

    /// The field table for the selected segment, as both panes see it.
    fn rows(&self) -> Vec<FieldRow> {
        segment_rows(
            self.msg(),
            self.seg_index(),
            self.report(),
            RowOptions {
                show_empty: self.show_empty,
                include_info: self.verbose,
            },
        )
    }

    fn finding_len(&self) -> usize {
        self.visible_findings().len()
    }

    fn visible_findings(&self) -> Vec<&crate::validate::Finding> {
        let index = self.seg_index();
        self.report()
            .findings
            .iter()
            .filter(|f| self.verbose || f.severity != Severity::Info)
            .filter(|f| self.all_findings || f.segment_index == Some(index))
            .collect()
    }
}

pub fn run(items: Vec<(String, Message, Report)>) -> io::Result<()> {
    // A panic inside the alternate screen would otherwise leave the terminal in
    // raw mode with no cursor.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        hook(info);
    }));
    let mut terminal = setup()?;
    let mut app = App::new(items);
    let result = event_loop(&mut terminal, &mut app);
    restore(&mut terminal)?;
    result
}

fn setup() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn restore(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if handle_key(app, key) {
            return Ok(());
        }
    }
}

/// Returns true when the app should exit.
fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    if app.help {
        app.help = false;
        return false;
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,
        KeyCode::Char('?') | KeyCode::F(1) => app.help = true,
        KeyCode::Down | KeyCode::Char('j') => app.step(1),
        KeyCode::Up | KeyCode::Char('k') => app.step(-1),
        KeyCode::PageDown => app.step(10),
        KeyCode::PageUp => app.step(-10),
        KeyCode::Home => app.step(-9999),
        KeyCode::End => app.step(9999),
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Segments => Focus::Fields,
                Focus::Fields => Focus::Findings,
                Focus::Findings => Focus::Segments,
            }
        }
        KeyCode::BackTab => {
            app.focus = match app.focus {
                Focus::Segments => Focus::Findings,
                Focus::Fields => Focus::Segments,
                Focus::Findings => Focus::Fields,
            }
        }
        KeyCode::Right | KeyCode::Char('l') => app.focus = Focus::Fields,
        KeyCode::Left | KeyCode::Char('h') => app.focus = Focus::Segments,
        KeyCode::Char('n') => app.move_message(1),
        KeyCode::Char('p') => app.move_message(-1),
        KeyCode::Char('a') => app.show_empty = !app.show_empty,
        KeyCode::Char('v') => app.verbose = !app.verbose,
        KeyCode::Char('r') => app.show_raw = !app.show_raw,
        KeyCode::Char('f') => app.all_findings = !app.all_findings,
        _ => {}
    }
    false
}

fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if app.show_raw { 5 } else { 4 }),
            Constraint::Min(6),
            Constraint::Length(8),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0]);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(30)])
        .split(chunks[1]);
    draw_segments(frame, app, panes[0]);
    draw_fields(frame, app, panes[1]);
    draw_findings(frame, app, chunks[2]);
    draw_footer(frame, app, chunks[3]);

    if app.help {
        draw_help(frame);
    }
}

fn draw_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let msg = app.msg();
    let report = app.report();
    let sep = &msg.sep;
    let version = msg.version();
    let desc = report.structure.map_or("", |s| s.desc);

    let mut spans = vec![
        Span::styled(
            if version.is_empty() {
                "HL7".to_string()
            } else {
                format!("HL7 v{version}")
            },
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            msg.type_label(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
    ];
    if !desc.is_empty() {
        spans.push(Span::raw("   "));
        spans.push(Span::styled(desc, Style::default().fg(MUTED)));
    }

    let mut meta: Vec<String> = Vec::new();
    if !msg.control_id().is_empty() {
        meta.push(msg.control_id());
    }
    if let Ok(ts) = crate::datetime::parse_ts(&msg.msh().comp(7, 1, sep)) {
        meta.push(ts.display());
    }
    let sending = msg.msh().comp(3, 1, sep);
    let receiving = msg.msh().comp(5, 1, sep);
    if !sending.is_empty() || !receiving.is_empty() {
        meta.push(format!("{sending} \u{2192} {receiving}"));
    }

    let mut lines = vec![
        Line::from(spans),
        Line::from(Span::styled(
            meta.join("  \u{b7}  "),
            Style::default().fg(MUTED),
        )),
    ];
    if app.show_raw {
        lines.push(Line::from(Span::styled(
            msg.segments[app.seg_index()].raw.clone(),
            Style::default().fg(MUTED),
        )));
    }

    let title = if app.items.len() > 1 {
        format!(
            " {}  \u{b7}  message {}/{} ",
            app.label(),
            app.current + 1,
            app.items.len()
        )
    } else {
        format!(" {} ", app.label())
    };
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MUTED))
                .title(Span::styled(title, Style::default().fg(ACCENT))),
        ),
        area,
    );
}

fn pane_block(title: &str, focused: bool) -> Block<'static> {
    let colour = if focused { ACCENT } else { MUTED };
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colour))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(colour).add_modifier(if focused {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ))
}

fn draw_segments(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let msg = &app.items[app.current].1;
    let report = &app.items[app.current].2;
    let items: Vec<ListItem<'_>> = msg
        .segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            let sev = report.segment_severity(i, app.verbose);
            let count = msg.find(&seg.name).len();
            let suffix = if count > 1 {
                format!(" {}/{}", seg.occurrence, count)
            } else {
                String::new()
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<4}", seg.name),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(render::glyph(sev), severity_style(sev)),
                Span::styled(suffix, Style::default().fg(MUTED)),
                Span::styled(
                    format!("  {}", spec::segment_desc(&seg.name).unwrap_or("")),
                    Style::default().fg(MUTED),
                ),
            ]))
        })
        .collect();

    let focused = app.focus == Focus::Segments;
    let list = List::new(items)
        .block(pane_block("Segments", focused))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 44, 52))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("\u{25b8} ");
    frame.render_stateful_widget(list, area, &mut app.segments);
}

fn draw_fields(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let seg_index = app.seg_index();
    let rows = app.rows();
    let seg = &app.items[app.current].1.segments[seg_index];
    let label_width = rows
        .iter()
        .map(|r| r.label.chars().count())
        .max()
        .unwrap_or(12)
        .clamp(12, 30);
    let value_width = (area.width as usize)
        .saturating_sub(label_width + 14)
        .max(12);

    let items: Vec<ListItem<'_>> = rows
        .iter()
        .map(|row| field_item(row, label_width, value_width))
        .collect();

    let title = format!(
        "{}  {}   line {}",
        seg.name,
        spec::segment_desc(&seg.name).unwrap_or(""),
        seg.line
    );
    let focused = app.focus == Focus::Fields;
    let list = List::new(items)
        .block(pane_block(&title, focused))
        .highlight_style(Style::default().bg(Color::Rgb(40, 44, 52)))
        .highlight_symbol("");
    frame.render_stateful_widget(list, area, &mut app.fields);
}

/// Paints one field row: status glyph, sequence, label, value, decoded value.
fn field_item(row: &FieldRow, label_width: usize, value_width: usize) -> ListItem<'static> {
    let label = match row.repetition {
        None => row.label.clone(),
        Some(n) => format!("  ~ rep {n}"),
    };
    let mut spans = vec![
        Span::styled(
            format!("{} ", render_glyph_or_space(row.severity)),
            severity_style(row.severity),
        ),
        Span::styled(format!("{:>3} ", row.seq), Style::default().fg(MUTED)),
    ];
    if row.present {
        spans.push(Span::styled(fit(&label, label_width + 1), Style::default()));
        spans.push(Span::styled(
            truncate(&row.value, value_width),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        if let Some(decoded) = &row.decoded {
            spans.push(Span::styled(
                format!("  \u{203a} {}", truncate(decoded, value_width)),
                Style::default().fg(MUTED),
            ));
        }
    } else {
        spans.push(Span::styled(
            fit(&label, label_width + 1),
            Style::default().fg(MUTED),
        ));
        let note = row.empty_note();
        let text = if note.is_empty() {
            "(empty)".to_string()
        } else {
            format!("(empty)  {note}")
        };
        spans.push(Span::styled(text, Style::default().fg(MUTED)));
    }
    ListItem::new(Line::from(spans))
}

const fn render_glyph_or_space(sev: Option<Severity>) -> &'static str {
    match sev {
        Some(s) => s.glyph(),
        None => " ",
    }
}

fn draw_findings(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let findings = app.visible_findings();
    let scope = if app.all_findings {
        "all segments"
    } else {
        "this segment"
    };
    let title = format!(
        "Validation \u{b7} {} \u{b7} {} error(s), {} warning(s)",
        scope,
        app.report().errors(),
        app.report().warnings()
    );
    let items: Vec<ListItem<'_>> = if findings.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            format!(" {} nothing to report", render::OK),
            Style::default().fg(Color::Green),
        )))]
    } else {
        findings
            .iter()
            .map(|f| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {} ", f.severity.glyph()),
                        severity_style(Some(f.severity)),
                    ),
                    Span::styled(
                        format!("{:<10}", f.location),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(f.summary.clone()),
                    Span::styled(
                        format!("  \u{2014} {}", f.detail),
                        Style::default().fg(MUTED),
                    ),
                ]))
            })
            .collect()
    };
    let focused = app.focus == Focus::Findings;
    let list = List::new(items)
        .block(pane_block(&title, focused))
        .highlight_style(Style::default().bg(Color::Rgb(40, 44, 52)))
        .highlight_symbol("");
    frame.render_stateful_widget(list, area, &mut app.findings);
}

fn draw_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let toggles = format!(
        "empty:{}  notes:{}  raw:{}  scope:{}",
        on_off(app.show_empty),
        on_off(app.verbose),
        on_off(app.show_raw),
        if app.all_findings { "all" } else { "segment" }
    );
    let keys = if app.items.len() > 1 {
        "\u{2191}\u{2193} move  tab pane  n/p message  a empty  v notes  r raw  f scope  ? help  q quit"
    } else {
        "\u{2191}\u{2193} move  tab pane  a empty  v notes  r raw  f scope  ? help  q quit"
    };
    let line = Line::from(vec![
        Span::styled(format!(" {keys}"), Style::default().fg(MUTED)),
        Span::raw("   "),
        Span::styled(toggles, Style::default().fg(ACCENT)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

const fn on_off(v: bool) -> &'static str {
    if v {
        "on"
    } else {
        "off"
    }
}

fn draw_help(frame: &mut Frame<'_>) {
    let area = centered(62, 17, frame.area());
    let text = vec![
        Line::from(Span::styled(
            "hl7probe interactive viewer",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  \u{2191}/\u{2193}, j/k    move within the focused pane"),
        Line::from("  \u{2190}/\u{2192}, h/l    jump between segments and fields"),
        Line::from("  tab          cycle segments \u{2192} fields \u{2192} validation"),
        Line::from("  n / p        next / previous message in the file"),
        Line::from("  a            show fields the sender left empty"),
        Line::from("  v            include informational notes"),
        Line::from("  r            show the raw segment line"),
        Line::from("  f            findings for this segment or the whole message"),
        Line::from("  q, esc       quit"),
        Line::from(""),
        Line::from(Span::styled(
            "  \u{2713} clean    \u{26a0} warning    \u{2717} error",
            Style::default().fg(MUTED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  press any key to close",
            Style::default().fg(MUTED),
        )),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(ACCENT)),
            ),
        area,
    );
}

fn centered(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width.saturating_sub(2));
    let h = height.min(area.height.saturating_sub(2));
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "panicking is the failure mode a test wants"
    )]

    use super::*;
    use crate::parser::parse_str;
    use crate::validate::validate;
    use ratatui::backend::TestBackend;
    use std::fmt::Write as _;

    const MSG: &str = "MSH|^~\\&|HIS|MERCY|LIS|LAB|20240115143200||ADT^A01^ADT_A01|MSG1|P|2.5.1\r\
EVN|A01|20240115143200||||20240115143000\r\
PID|1||123456^^^MERCY^MR||Smith^John^A||19850332|M\r\
PV1|1|I|ER^101^A|E|||1234^Adams^Alice||||||||||||V1\r";

    fn app() -> App {
        let msg = parse_str(MSG);
        let report = validate(&msg);
        App::new(vec![("test.hl7".to_string(), msg, report)])
    }

    /// A second message, so the wrap-around in `move_message` has somewhere to go.
    fn two_message_app() -> App {
        let items = ["MSG1", "MSG2"]
            .iter()
            .map(|id| {
                let msg = parse_str(&MSG.replace("MSG1", id));
                let report = validate(&msg);
                ((*id).to_string(), msg, report)
            })
            .collect();
        App::new(items)
    }

    #[test]
    fn message_navigation_wraps_in_both_directions() {
        let mut batch = two_message_app();
        assert_eq!(batch.current, 0);
        batch.move_message(1);
        assert_eq!(batch.current, 1);
        batch.move_message(1); // past the end, back to the first
        assert_eq!(batch.current, 0);
        batch.move_message(-1); // before the start, on to the last
        assert_eq!(batch.current, 1);

        // A single message has nowhere to go, and must not divide by zero.
        let mut lone = app();
        lone.move_message(1);
        lone.move_message(-1);
        assert_eq!(lone.current, 0);
    }

    #[test]
    fn list_navigation_saturates_at_both_ends() {
        let mut app = app();
        let last = app.items[0].1.segments.len() - 1;
        app.step(-1); // already at the top
        assert_eq!(app.segments.selected(), Some(0));
        app.step(9999); // End
        assert_eq!(app.segments.selected(), Some(last));
        app.step(10); // PageDown past the end
        assert_eq!(app.segments.selected(), Some(last));
        app.step(-9999); // Home
        assert_eq!(app.segments.selected(), Some(0));
        app.step(-10); // PageUp past the start
        assert_eq!(app.segments.selected(), Some(0));
    }

    fn screen(app: &mut App) -> String {
        let mut terminal = Terminal::new(TestBackend::new(120, 34)).unwrap();
        terminal.draw(|frame| draw(frame, app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<Vec<_>>()
            .chunks(120)
            .map(<[&str]>::concat)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Renders a frame to ANSI text for manual inspection and for the images in
    /// the README: `cargo test -- --ignored --nocapture dump_screen`.
    #[test]
    #[ignore = "visual check, not an assertion"]
    fn dump_screen() {
        let path =
            std::env::var("HL7TEST_DUMP").unwrap_or_else(|_| "examples/invalid.hl7".to_string());
        let steps: isize = std::env::var("HL7TEST_DUMP_STEP")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let msg = parse_str(&std::fs::read_to_string(&path).unwrap());
        let report = validate(&msg);
        let name = std::path::Path::new(&path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let mut app = App::new(vec![(name, msg, report)]);
        app.step(steps);
        app.focus = Focus::Fields;
        println!("{}", ansi_screen(&mut app));
    }

    /// Re-emits a rendered buffer as ANSI so the colours survive the dump.
    fn ansi_screen(app: &mut App) -> String {
        let height: u16 = std::env::var("HL7TEST_DUMP_ROWS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(34);
        let width = 120u16;
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, app)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..height {
            for x in 0..width {
                let cell = &buffer[(x, y)];
                let mut codes: Vec<String> = Vec::new();
                if let Some(code) = fg_code(cell.fg) {
                    codes.push(code);
                }
                if cell.modifier.contains(Modifier::BOLD) {
                    codes.push("1".into());
                }
                let _ = write!(out, "\u{1b}[0;{}m{}", codes.join(";"), cell.symbol());
            }
            out.push_str("\u{1b}[0m\n");
        }
        out
    }

    fn fg_code(colour: Color) -> Option<String> {
        Some(match colour {
            Color::Red => "31".into(),
            Color::Green => "32".into(),
            Color::Yellow => "33".into(),
            Color::Cyan => "36".into(),
            Color::DarkGray => "90".into(),
            Color::Rgb(r, g, b) => format!("38;2;{r};{g};{b}"),
            _ => return None,
        })
    }

    #[test]
    fn draws_header_segments_fields_and_findings() {
        let mut app = app();
        let text = screen(&mut app);
        assert!(text.contains("test.hl7"));
        assert!(text.contains("MSG1"), "control id line is not clipped");
        assert!(text.contains("HL7 v2.5.1"));
        assert!(text.contains("ADT^A01"));
        assert!(text.contains("MSH"), "segment list");
        assert!(
            text.contains("Message Header"),
            "field pane follows the selection"
        );
        assert!(text.contains("PID-7"), "findings pane");
        assert!(text.contains("quit"), "footer hints");
    }

    #[test]
    fn moving_down_the_segment_list_changes_the_field_pane() {
        let mut app = app();
        app.step(2);
        let text = screen(&mut app);
        assert_eq!(app.seg_index(), 2);
        assert!(text.contains("Patient Identifier List"));
        assert!(text.contains("Smith^John^A"));
        assert!(text.contains("John A Smith"), "decoded value");
    }

    #[test]
    fn tab_cycles_focus_and_q_exits() {
        let mut app = app();
        assert!(!handle_key(&mut app, KeyEvent::from(KeyCode::Tab)));
        assert!(app.focus == Focus::Fields);
        handle_key(&mut app, KeyEvent::from(KeyCode::Tab));
        assert!(app.focus == Focus::Findings);
        handle_key(&mut app, KeyEvent::from(KeyCode::Tab));
        assert!(app.focus == Focus::Segments);
        assert!(handle_key(&mut app, KeyEvent::from(KeyCode::Char('q'))));
        assert!(handle_key(&mut app, KeyEvent::from(KeyCode::Esc)));
    }

    #[test]
    fn toggles_change_what_is_rendered() {
        let mut app = app();
        app.step(2); // PID
        assert!(!screen(&mut app).contains("Mother's Maiden Name"));
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('a')));
        assert!(
            screen(&mut app).contains("Mother's Maiden Name"),
            "'a' reveals empty fields"
        );

        handle_key(&mut app, KeyEvent::from(KeyCode::Char('r')));
        assert!(
            screen(&mut app).contains("PID|1||123456"),
            "'r' shows the raw line"
        );

        assert!(app.all_findings);
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('f')));
        assert!(!app.all_findings);
        assert!(screen(&mut app).contains("this segment"));
    }

    #[test]
    fn help_overlay_opens_and_closes() {
        let mut app = app();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('?')));
        assert!(app.help);
        assert!(screen(&mut app).contains("interactive viewer"));
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('x')));
        assert!(!app.help, "any key dismisses help");
    }

    #[test]
    fn n_and_p_walk_through_a_batch() {
        let one = parse_str(MSG);
        let two = parse_str(&MSG.replace("ADT^A01^ADT_A01|MSG1", "ADT^A03^ADT_A03|MSG2"));
        let mut app = App::new(vec![
            ("batch.hl7".into(), one, validate(&parse_str(MSG))),
            ("batch.hl7".into(), two, validate(&parse_str(MSG))),
        ]);
        assert_eq!(app.current, 0);
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('n')));
        assert_eq!(app.current, 1);
        assert!(screen(&mut app).contains("message 2/2"));
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('n')));
        assert_eq!(app.current, 0, "wraps around");
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('p')));
        assert_eq!(app.current, 1);
    }

    #[test]
    fn selection_never_runs_past_the_ends() {
        let mut app = app();
        app.step(-5);
        assert_eq!(app.segments.selected(), Some(0));
        app.step(500);
        assert_eq!(app.segments.selected(), Some(3));
        assert_eq!(app.seg_index(), 3);
    }
}
