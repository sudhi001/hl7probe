//! Terminal rendering of a parsed message and its validation report.

use std::fmt::Write as _;

use crate::datetime;
use crate::parser::{Message, Segment};
use crate::spec;
use crate::text::{pad, truncate};
use crate::validate::{Category, Finding, Report, Severity};
use crate::view::{segment_rows, FieldRow, RowOptions};
use owo_colors::OwoColorize;

pub const RULE: char = '\u{2500}'; // ─
pub const OK: &str = "\u{2713}"; // ✓

#[derive(Clone, Copy)]
pub struct Paint {
    pub enabled: bool,
}

/// The methods take `self` by value: `Paint` is one byte, and a `&Paint`
/// receiver would resolve to `OwoColorize`'s blanket impl instead of these.
impl Paint {
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }
    pub fn dim(self, s: &str) -> String {
        if self.enabled {
            s.dimmed().to_string()
        } else {
            s.to_string()
        }
    }
    pub fn bold(self, s: &str) -> String {
        if self.enabled {
            s.bold().to_string()
        } else {
            s.to_string()
        }
    }
    pub fn red(self, s: &str) -> String {
        if self.enabled {
            s.red().to_string()
        } else {
            s.to_string()
        }
    }
    pub fn yellow(self, s: &str) -> String {
        if self.enabled {
            s.yellow().to_string()
        } else {
            s.to_string()
        }
    }
    pub fn green(self, s: &str) -> String {
        if self.enabled {
            s.green().to_string()
        } else {
            s.to_string()
        }
    }
    pub fn cyan(self, s: &str) -> String {
        if self.enabled {
            s.cyan().to_string()
        } else {
            s.to_string()
        }
    }
    pub fn severity(self, sev: Severity, s: &str) -> String {
        match sev {
            Severity::Error => self.red(s),
            Severity::Warning => self.yellow(s),
            Severity::Info => self.dim(s),
        }
    }
}

pub struct Options {
    pub paint: Paint,
    pub width: usize,
    /// Show fields the sender left empty.
    pub show_empty: bool,
    /// Show info-level findings.
    pub verbose: bool,
    /// Skip the per-segment field tables.
    pub summary_only: bool,
    /// Restrict field tables to these segment names.
    pub only: Vec<String>,
    /// Print the raw segment text above each table.
    pub raw: bool,
}

pub const fn glyph(sev: Option<Severity>) -> &'static str {
    match sev {
        None => OK,
        Some(s) => s.glyph(),
    }
}

fn paint_status(p: Paint, sev: Option<Severity>) -> String {
    match sev {
        None => p.green(OK),
        Some(s) => p.severity(s, s.glyph()),
    }
}

fn rule(p: Paint, width: usize) -> String {
    p.dim(&RULE.to_string().repeat(width))
}

/// Header block: version, message type, routing and identifiers.
pub fn message_header(msg: &Message, report: &Report, o: &Options) -> String {
    let p = o.paint;
    let sep = &msg.sep;
    let mut out = String::new();
    let version = msg.version();
    let version_label = if version.is_empty() {
        "HL7 (no version)".to_string()
    } else {
        format!("HL7 v{version}")
    };
    let type_label = msg.type_label();
    let desc = report
        .structure
        .map(|s| s.desc.to_string())
        .or_else(|| spec::trigger_desc(&msg.message_type().1).map(ToString::to_string))
        .unwrap_or_default();

    let _ = writeln!(
        out,
        "{}   {}{}",
        p.bold(&version_label),
        p.cyan(&p.bold(&type_label)),
        if desc.is_empty() {
            String::new()
        } else {
            format!("   {}", p.dim(&desc))
        }
    );

    let mut meta: Vec<String> = Vec::new();
    let control = msg.control_id();
    if !control.is_empty() {
        meta.push(control);
    }
    if let Ok(ts) = datetime::parse_ts(&msg.msh().comp(7, 1, sep)) {
        meta.push(ts.display());
    }
    let sending = join_app(&msg.msh().comp(3, 1, sep), &msg.msh().comp(4, 1, sep));
    let receiving = join_app(&msg.msh().comp(5, 1, sep), &msg.msh().comp(6, 1, sep));
    if !sending.is_empty() || !receiving.is_empty() {
        meta.push(format!(
            "{} \u{2192} {}",
            if sending.is_empty() {
                "?".into()
            } else {
                sending
            },
            if receiving.is_empty() {
                "?".into()
            } else {
                receiving
            }
        ));
    }
    let processing = msg.msh().comp(11, 1, sep);
    if let Some(meaning) = spec::code_meaning("0103", &processing) {
        meta.push(meaning.to_string());
    }
    if !meta.is_empty() {
        let _ = writeln!(out, "{}", p.dim(&meta.join("  \u{b7}  ")));
    }
    out
}

fn join_app(app: &str, facility: &str) -> String {
    match (app.is_empty(), facility.is_empty()) {
        (true, true) => String::new(),
        (false, true) => app.to_string(),
        (true, false) => facility.to_string(),
        (false, false) => format!("{app}/{facility}"),
    }
}

/// The `MSH ✓ / EVN ✓ / PID ⚠` overview.
pub fn segment_overview(msg: &Message, report: &Report, o: &Options) -> String {
    let p = o.paint;
    let mut out = String::new();
    let _ = writeln!(out, "{}", p.bold("Segments"));
    let _ = writeln!(out, "{}", rule(p, o.width.min(60)));
    for (i, seg) in msg.segments.iter().enumerate() {
        let sev = report.segment_severity(i, o.verbose);
        let desc = spec::segment_desc(&seg.name).map_or_else(
            || {
                if seg.is_custom() {
                    "site-defined segment".into()
                } else {
                    "unknown segment".into()
                }
            },
            ToString::to_string,
        );
        let repeat = msg.find(&seg.name).len();
        let occurrence = if repeat > 1 {
            p.dim(&format!(" ({}/{})", seg.occurrence, repeat))
        } else {
            String::new()
        };
        let _ = writeln!(
            out,
            "{} {}{}  {}",
            p.bold(&seg.name),
            paint_status(p, sev),
            occurrence,
            p.dim(&desc)
        );
    }
    out
}

/// Column widths for a segment's field table.
struct Columns {
    label: usize,
    value: usize,
}

impl Columns {
    fn for_rows(rows: &[FieldRow], width: usize) -> Self {
        let label = rows
            .iter()
            .map(|r| r.label.chars().count())
            .max()
            .unwrap_or(10)
            .clamp(10, 34)
            + 2;
        Self {
            label,
            value: width.saturating_sub(label + 10).max(16),
        }
    }
}

/// Formats one row: `⚠  11  Patient Address   123 Main St   › decoded`.
fn format_row(row: &FieldRow, columns: &Columns, o: &Options) -> String {
    let p = o.paint;
    let status = match row.severity {
        Some(s) => p.severity(s, s.glyph()),
        None => " ".to_string(),
    };
    let seq_cell = match row.repetition {
        None => p.dim(&format!("{:>3}", row.seq)),
        Some(_) => "   ".to_string(),
    };
    let label = match row.repetition {
        None => truncate(&row.label, columns.label - 1),
        Some(n) => format!("~ rep {n}"),
    };

    if !row.present {
        let note = row.empty_note();
        let suffix = if note.is_empty() {
            "(empty)".to_string()
        } else {
            format!("(empty)  {note}")
        };
        return format!(
            "{} {} {}{}",
            status,
            seq_cell,
            pad(&p.dim(&label), columns.label + p.dim("").len()),
            p.dim(&suffix)
        );
    }

    let value = truncate(&row.value, columns.value);
    let mut out = format!(
        "{} {} {}{}",
        status,
        seq_cell,
        pad(&label, columns.label),
        value
    );
    if let Some(decoded) = &row.decoded {
        let room = o
            .width
            .saturating_sub(8 + columns.label + value.chars().count());
        if room > 12 {
            let _ = write!(
                out,
                "  {}",
                p.dim(&format!(
                    "\u{203a} {}",
                    truncate(decoded, room.saturating_sub(3))
                ))
            );
        }
    }
    out
}

/// The detail table for one segment occurrence.
pub fn segment_detail(
    msg: &Message,
    seg: &Segment,
    index: usize,
    report: &Report,
    o: &Options,
) -> String {
    let p = o.paint;
    let mut out = String::new();
    let desc = spec::segment_desc(&seg.name).unwrap_or("");
    let count = msg.find(&seg.name).len();
    let title = if count > 1 {
        format!("{} ({} of {})", seg.name, seg.occurrence, count)
    } else {
        seg.name.clone()
    };
    let heading = if desc.is_empty() {
        p.bold(&title)
    } else {
        format!("{} {} {}", p.bold(&title), p.dim("\u{b7}"), p.dim(desc))
    };
    let _ = writeln!(
        out,
        "{}{}",
        heading,
        p.dim(&format!("   line {}", seg.line))
    );
    let _ = writeln!(out, "{}", rule(p, o.width.min(72)));
    if o.raw {
        let _ = writeln!(out, "{}", p.dim(&truncate(&seg.raw, o.width)));
    }

    let rows = segment_rows(
        msg,
        index,
        report,
        RowOptions {
            show_empty: o.show_empty,
            include_info: o.verbose,
        },
    );
    if rows.is_empty() {
        let _ = writeln!(out, "{}", p.dim("(no populated fields)"));
    }
    let columns = Columns::for_rows(&rows, o.width);
    for row in &rows {
        out.push_str(&format_row(row, &columns, o));
        out.push('\n');
    }
    out
}

/// Validation summary: one line per check category, then the findings.
pub fn validation(report: &Report, o: &Options) -> String {
    let p = o.paint;
    let mut out = String::new();
    let _ = writeln!(out, "{}", p.bold("Validation"));
    let _ = writeln!(out, "{}", rule(p, o.width.min(60)));

    for cat in Category::ALL {
        let sev = report
            .findings
            .iter()
            .filter(|f| f.category == cat)
            .filter(|f| o.verbose || f.severity != Severity::Info)
            .map(|f| f.severity)
            .max();
        let note = match cat {
            Category::Structure => report
                .structure
                .map_or_else(|| "no profile matched".into(), |s| s.id.to_string()),
            _ => String::new(),
        };
        if note.is_empty() {
            let _ = writeln!(out, "{} {}", paint_status(p, sev), cat.title());
        } else {
            let _ = writeln!(
                out,
                "{} {}{}",
                paint_status(p, sev),
                pad(cat.title(), 18),
                p.dim(&note)
            );
        }
    }

    let shown: Vec<&Finding> = report
        .findings
        .iter()
        .filter(|f| o.verbose || f.severity != Severity::Info)
        .collect();
    if !shown.is_empty() {
        out.push('\n');
    }
    let loc_width = shown
        .iter()
        .map(|f| f.location.chars().count())
        .max()
        .unwrap_or(6)
        .clamp(6, 14);
    for f in &shown {
        let head = format!(
            "{} {} {}",
            p.severity(f.severity, f.severity.glyph()),
            pad(&f.location, loc_width),
            f.summary
        );
        let room = o.width.saturating_sub(head.chars().count() + 4);
        let detail = if room > 20 && !f.detail.is_empty() {
            p.dim(&format!("  \u{2014} {}", truncate(&f.detail, room)))
        } else {
            String::new()
        };
        let _ = writeln!(out, "{head}{detail}");
    }
    out
}

pub fn summary_line(report: &Report, o: &Options) -> String {
    let p = o.paint;
    let errors = report.errors();
    let warnings = report.warnings();
    let infos = report.count(Severity::Info);
    let mut parts: Vec<String> = Vec::new();
    if errors > 0 {
        parts.push(p.red(&plural(errors, "error")));
    }
    if warnings > 0 {
        parts.push(p.yellow(&plural(warnings, "warning")));
    }
    if infos > 0 && o.verbose {
        parts.push(p.dim(&plural(infos, "note")));
    }
    if parts.is_empty() {
        let hidden = if infos > 0 && !o.verbose {
            p.dim(&format!(
                "  ({} note{} hidden, use -v)",
                infos,
                if infos == 1 { "" } else { "s" }
            ))
        } else {
            String::new()
        };
        return format!(
            "{} {}{}",
            p.green(OK),
            p.green("message passes all checks"),
            hidden
        );
    }
    let hidden = if infos > 0 && !o.verbose {
        p.dim(&format!(
            "  ({} note{} hidden, use -v)",
            infos,
            if infos == 1 { "" } else { "s" }
        ))
    } else {
        String::new()
    };
    format!("{}{}", parts.join(p.dim("  \u{b7}  ").as_str()), hidden)
}

fn plural(n: usize, word: &str) -> String {
    format!("{} {}{}", n, word, if n == 1 { "" } else { "s" })
}

/// Full report for one message.
pub fn render_message(msg: &Message, report: &Report, o: &Options) -> String {
    let mut out = String::new();
    out.push_str(&message_header(msg, report, o));
    out.push('\n');
    out.push_str(&segment_overview(msg, report, o));
    if !o.summary_only {
        for (i, seg) in msg.segments.iter().enumerate() {
            if !o.only.is_empty() && !o.only.iter().any(|n| n.eq_ignore_ascii_case(&seg.name)) {
                continue;
            }
            out.push('\n');
            out.push_str(&segment_detail(msg, seg, i, report, o));
        }
    }
    out.push('\n');
    out.push_str(&validation(report, o));
    out.push('\n');
    out.push_str(&summary_line(report, o));
    out.push('\n');
    out
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

    const MSG: &str = "MSH|^~\\&|HIS|MERCY|LIS|LAB|20240115143200||ADT^A01^ADT_A01|MSG1|P|2.5.1\r\
EVN|A01|20240115143200||||20240115143000\r\
PID|1||123456^^^MERCY^MR||Smith^John^A^^Mr||19850312|M|||42 Oak St^^Springfield^IL^62704\r\
PV1|1|I|ER^101^A^MERCY|E|||1234^Adams^Alice^^^Dr||||||||||||V1|||||||||||||||||||||||||20240115143000\r";

    fn options() -> Options {
        Options {
            paint: Paint::new(false),
            width: 100,
            show_empty: false,
            verbose: false,
            summary_only: false,
            only: Vec::new(),
            raw: false,
        }
    }

    #[test]
    fn report_shows_the_message_type_segments_and_verdict() {
        let msg = parse_str(MSG);
        let report = validate(&msg);
        let text = render_message(&msg, &report, &options());
        assert!(text.contains("HL7 v2.5.1"));
        assert!(text.contains("ADT^A01"));
        assert!(text.contains("Admit / Visit Notification"));
        for segment in ["MSH", "EVN", "PID", "PV1"] {
            assert!(text.contains(segment), "missing {segment}");
        }
        assert!(text.contains("Patient Identifier List"));
        assert!(text.contains("message passes all checks"));
    }

    #[test]
    fn findings_are_listed_with_their_location() {
        let msg = parse_str(
            &MSG.replace("19850312", "19850332")
                .replace("|ER^101^A^MERCY|", "|^101^A|"),
        );
        let report = validate(&msg);
        let text = render_message(&msg, &report, &options());
        assert!(text.contains("PID-7"));
        assert!(text.contains("not a valid date/time"));
        assert!(text.contains("PV1-3"));
        assert!(text.contains("invalid location"));
        assert!(
            text.contains("2 errors"),
            "{}",
            summary_line(&report, &options())
        );
    }

    #[test]
    fn empty_recommended_fields_are_visible_but_optional_ones_are_not() {
        let msg =
            parse_str("MSH|^~\\&|HIS|MERCY|LIS|LAB|20240115143200||ADT^A01|MSG1|P|2.5.1\rPID|1\r");
        let report = validate(&msg);
        let index = msg.segments.iter().position(|s| s.name == "PID").unwrap();
        let text = segment_detail(&msg, &msg.segments[index], index, &report, &options());
        assert!(
            text.contains("Patient Address"),
            "recommended fields stay visible"
        );
        assert!(
            !text.contains("Birth Order"),
            "optional empty fields stay hidden"
        );

        let mut all = options();
        all.show_empty = true;
        let text = segment_detail(&msg, &msg.segments[index], index, &report, &all);
        assert!(
            text.contains("Birth Order"),
            "-a reveals every defined field"
        );
    }

    #[test]
    fn colour_can_be_switched_off_completely() {
        let msg = parse_str(MSG);
        let report = validate(&msg);
        let plain = render_message(&msg, &report, &options());
        assert!(
            !plain.contains('\u{1b}'),
            "no escape codes when colour is disabled"
        );

        let mut coloured = options();
        coloured.paint = Paint::new(true);
        assert!(render_message(&msg, &report, &coloured).contains('\u{1b}'));
    }

    #[test]
    fn long_values_are_truncated_to_the_terminal_width() {
        let long = "X".repeat(400);
        let msg = parse_str(&format!(
            "MSH|^~\\&|HIS|MERCY|LIS|LAB|20240115143200||ADT^A01|MSG1|P|2.5.1\rPID|1||1^^^A^MR||{long}\r"
        ));
        let report = validate(&msg);
        let mut narrow = options();
        narrow.width = 80;
        let text = segment_detail(&msg, &msg.segments[1], 1, &report, &narrow);
        assert!(
            text.lines().all(|l| l.chars().count() <= 100),
            "rows should stay near the width"
        );
    }
}
