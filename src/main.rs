//! hl7probe - inspect and validate HL7 v2 messages from the command line.

mod datetime;
mod parser;
mod render;
mod spec;
mod text;
mod tui;
mod validate;
mod view;

use std::cell::Cell;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{ArgAction, Parser, ValueEnum};
use serde::ser::{SerializeMap as _, SerializeSeq as _};
use serde::{Serialize, Serializer};

use parser::Message;
use validate::{Report, Severity};

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Parser)]
#[command(
    name = "hl7probe",
    version,
    about = "Inspect and validate HL7 v2 messages",
    long_about = "Reads HL7 v2.x messages, decodes them into named fields and reports \
structural, required-field, data-type, code-table and consistency problems.\n\n\
Exit status: 0 clean, 1 validation errors, 2 unreadable input.",
    after_help = "EXAMPLES:\n  \
hl7probe adt.hl7                    inspect a message\n  \
hl7probe --tui adt.hl7              browse it interactively\n  \
hl7probe -s PID,PV1 adt.hl7         only show two segments\n  \
hl7probe -f PID-5.1 adt.hl7         print one field value\n  \
hl7probe --json adt.hl7 | jq .      machine-readable report\n  \
cat adt.hl7 | hl7probe -q           validate in a pipeline"
)]
struct Cli {
    /// HL7 files to read; "-" or no argument reads standard input
    #[arg(value_name = "FILE")]
    files: Vec<PathBuf>,

    /// Browse the message in an interactive terminal viewer
    #[arg(short = 't', long)]
    tui: bool,

    /// Emit the full report as JSON
    #[arg(long, conflicts_with = "tui")]
    json: bool,

    /// Print only the validation verdict
    #[arg(short = 'q', long, conflicts_with = "json")]
    quiet: bool,

    /// Include informational notes
    #[arg(short = 'v', long, action = ArgAction::SetTrue)]
    verbose: bool,

    /// Show fields the sender left empty
    #[arg(short = 'a', long = "all")]
    show_empty: bool,

    /// Limit field tables to these segments (comma separated)
    #[arg(
        short = 's',
        long = "segment",
        value_delimiter = ',',
        value_name = "NAME"
    )]
    segments: Vec<String>,

    /// Skip field tables and show only the segment list and verdict
    #[arg(long)]
    summary: bool,

    /// Print the raw segment line above each field table
    #[arg(long)]
    raw: bool,

    // The text is clap's --help output, so the brackets have to stay literal
    // rather than become an intra-doc link.
    #[allow(rustdoc::broken_intra_doc_links, reason = "OBX[2] is help text")]
    /// Print a single value, e.g. PID-5.1, OBX[2]-5 or PV1-3.2
    #[arg(short = 'f', long = "field", value_name = "PATH")]
    field: Option<String>,

    /// Treat warnings as errors when setting the exit status
    #[arg(long)]
    strict: bool,

    /// Only inspect the Nth message in the file
    #[arg(short = 'm', long, value_name = "N")]
    message: Option<usize>,

    /// When to colourise output
    #[arg(long, value_enum, default_value = "auto")]
    color: ColorChoice,

    /// Wrap output at this many columns
    #[arg(long, value_name = "COLUMNS")]
    width: Option<usize>,
}

struct Input {
    label: String,
    text: String,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("hl7probe: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: &Cli) -> Result<ExitCode, String> {
    let inputs = read_inputs(cli)?;
    let color = match cli.color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
    };
    let width = cli
        .width
        .or_else(|| crossterm::terminal::size().ok().map(|(w, _)| w as usize))
        .unwrap_or(100)
        .clamp(48, 160);

    let options = render::Options {
        paint: render::Paint::new(color),
        width,
        show_empty: cli.show_empty,
        verbose: cli.verbose,
        summary_only: cli.summary,
        only: cli.segments.clone(),
        raw: cli.raw,
    };

    // Every mode works from the split-but-unparsed messages. Only the viewer,
    // which has to let you page back and forth, parses the whole batch; the
    // rest parse, use and drop one message at a time, so peak memory follows
    // the largest message rather than the size of the file.
    let mut files: Vec<ScannedFile> = Vec::new();
    for input in inputs {
        files.push(scan_file(input, cli.message)?);
    }

    if let Some(path) = &cli.field {
        return query_field(&files, path, &options);
    }
    if cli.tui {
        let mut all: Vec<(String, Message, Report)> = Vec::new();
        for file in &files {
            for raw in file.selected() {
                if let Ok(msg) = parser::parse_message(raw) {
                    let report = validate::validate(&msg);
                    all.push((file.label.clone(), msg, report));
                }
            }
        }
        if all.is_empty() {
            return Err("no parsable messages to display".into());
        }
        tui::run(all).map_err(|e| e.to_string())?;
        return Ok(ExitCode::SUCCESS);
    }
    if cli.json {
        return emit_json(&files, cli.strict);
    }

    let mut out = Output::stdout();
    let verdict = write_files(&files, &options, cli.quiet, &mut out)
        .and_then(|v| out.finish().map(|()| v))
        .map_err(|e| format!("stdout: {e}"))?;
    Ok(exit_code(verdict.worst, verdict.parse_failures, cli.strict))
}

/// What a run of the text report means for the exit status.
struct Verdict {
    worst: Option<Severity>,
    parse_failures: usize,
}

/// Writes every message of every file, parsing each one only when it is about
/// to be written and dropping it straight after.
fn write_files(
    files: &[ScannedFile],
    o: &render::Options,
    quiet: bool,
    out: &mut Output,
) -> io::Result<Verdict> {
    let mut worst: Option<Severity> = None;
    let mut parse_failures = 0usize;
    let multi_file = files.len() > 1;

    for file in files {
        if multi_file && !quiet {
            writeln!(
                out,
                "\n{}",
                o.paint.bold(&format!(
                    "{} {}",
                    render::RULE.to_string().repeat(2),
                    file.label
                ))
            )?;
        }
        if !quiet {
            for note in &file.notes {
                writeln!(out, "{}", o.paint.yellow(&format!("note: {note}")))?;
            }
        }
        // The scan already found these, which is what keeps them ahead of the
        // messages the way a buffered report used to put them.
        for error in &file.errors {
            parse_failures += 1;
            writeln!(out, "{}", o.paint.red(&format!("parse error: {error}")))?;
        }
        // Counted over the messages that parse, not over the raw ones, so an
        // unreadable message in the middle does not shift the numbering.
        let mut index = 0usize;
        for raw in file.selected() {
            let Ok(msg) = parser::parse_message(raw) else {
                continue; // Already reported above.
            };
            let report = validate::validate(&msg);
            worst = worst.max(report.worst());
            if quiet {
                writeln!(out, "{}", quiet_line(file, &msg, &report, index, o))?;
                index += 1;
                continue;
            }
            if file.message_count > 1 {
                writeln!(
                    out,
                    "\n{}",
                    o.paint.dim(&format!(
                        "message {} of {}   line {}",
                        index + 1,
                        file.message_count,
                        msg.start_line
                    ))
                )?;
            }
            for note in &msg.notes {
                writeln!(out, "{}", o.paint.dim(&format!("note: {note}")))?;
            }
            out.write_all(render::render_message(&msg, &report, o).as_bytes())?;
            index += 1;
        }
    }
    Ok(Verdict {
        worst,
        parse_failures,
    })
}

/// One grep-friendly line per message: where it came from, what it is, and the
/// verdict.
fn quiet_line(
    file: &ScannedFile,
    msg: &Message,
    report: &Report,
    index: usize,
    o: &render::Options,
) -> String {
    let origin = if file.message_count > 1 {
        format!("{}#{}", file.label, index + 1)
    } else {
        file.label.clone()
    };
    format!(
        "{}  {}  {}",
        o.paint.bold(&origin),
        msg.type_label(),
        render::summary_line(report, o)
    )
}

/// Buffered stdout that treats a closed pipe (`hl7probe ... | head`) as a
/// normal end. Writing stops there, but the walk over the messages carries on
/// so the exit status still reflects the whole input.
struct Output {
    inner: io::BufWriter<io::Stdout>,
    closed: bool,
}

impl Output {
    fn stdout() -> Self {
        Self {
            inner: io::BufWriter::new(io::stdout()),
            closed: false,
        }
    }

    fn finish(&mut self) -> io::Result<()> {
        self.flush()
    }
}

impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.closed {
            return Ok(buf.len());
        }
        match self.inner.write(buf) {
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                self.closed = true;
                Ok(buf.len())
            }
            other => other,
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }
        match self.inner.flush() {
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                self.closed = true;
                Ok(())
            }
            other => other,
        }
    }
}

fn exit_code(worst: Option<Severity>, parse_failures: usize, strict: bool) -> ExitCode {
    if parse_failures > 0 {
        return ExitCode::from(2);
    }
    match worst {
        Some(Severity::Error) => ExitCode::from(1),
        Some(Severity::Warning) if strict => ExitCode::from(1),
        _ => ExitCode::SUCCESS,
    }
}

/// A file split into messages but not yet parsed. Keeping the raw lines rather
/// than the parsed trees is what bounds memory on a large batch.
struct ScannedFile {
    label: String,
    raws: Vec<parser::RawMessage>,
    notes: Vec<String>,
    /// Found by the scan, so they still print ahead of the messages.
    errors: Vec<String>,
    /// How many of the selected messages parse, which decides whether the
    /// output numbers them.
    message_count: usize,
    /// The single message `-m` asked for, if any.
    only: Option<usize>,
}

impl ScannedFile {
    /// The raw messages this run reports on, which is all of them unless `-m`
    /// picked one.
    fn selected(&self) -> impl Iterator<Item = &parser::RawMessage> {
        selected(&self.raws, self.only)
    }
}

/// The messages `-m` leaves in play, as a free function so the scan can use it
/// before there is a `ScannedFile` to borrow.
fn selected(
    raws: &[parser::RawMessage],
    only: Option<usize>,
) -> impl Iterator<Item = &parser::RawMessage> {
    raws.iter()
        .enumerate()
        .filter(move |(i, _)| only.is_none_or(|n| i + 1 == n))
        .map(|(_, raw)| raw)
}

/// Splits a file into messages and checks which of them can be read, so parse
/// errors still print ahead of the messages. The check reads only each
/// message's MSH line, which is where both of the parser's failure modes live.
fn scan_file(input: Input, only: Option<usize>) -> Result<ScannedFile, String> {
    let Input { label, text } = input;
    let (raws, notes) = parser::split_messages(&text);
    drop(text);
    if raws.is_empty() {
        return Err(format!(
            "{label}: no MSH segment found - is this an HL7 v2 message?"
        ));
    }
    let mut errors = Vec::new();
    let mut message_count = 0usize;
    for raw in selected(&raws, only) {
        match raw.separators() {
            Ok(_) => message_count += 1,
            Err(e) => errors.push(format!("{label}: {e}")),
        }
    }
    if let Some(n) = only {
        if message_count == 0 && errors.is_empty() {
            return Err(format!(
                "{}: message {} requested but the file holds {}",
                label,
                n,
                raws.len()
            ));
        }
    }
    Ok(ScannedFile {
        label,
        raws,
        notes,
        errors,
        message_count,
        only,
    })
}

fn read_inputs(cli: &Cli) -> Result<Vec<Input>, String> {
    let use_stdin = cli.files.is_empty() || cli.files.iter().any(|f| f.as_os_str() == "-");
    if cli.files.is_empty() && io::stdin().is_terminal() {
        return Err("no input given - pass a file or pipe a message in (try --help)".into());
    }
    let mut inputs = Vec::new();
    for file in cli.files.iter().filter(|f| f.as_os_str() != "-") {
        inputs.push(Input {
            label: label_for(file),
            text: read_file(file)?,
        });
    }
    if use_stdin {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| format!("stdin: {e}"))?;
        inputs.push(Input {
            label: "<stdin>".into(),
            text: decode(buf),
        });
    }
    Ok(inputs)
}

fn label_for(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().to_string(),
    )
}

fn read_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    Ok(decode(bytes))
}

/// HL7 traffic is frequently latin-1; fall back to a lossy 8-bit decode so a
/// stray accented character never costs us the whole message.
fn decode(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => e.into_bytes().iter().map(|b| *b as char).collect(),
    }
}

// ------------------------------------------------------------------ field query

struct FieldPath {
    segment: String,
    occurrence: usize,
    field: usize,
    repetition: Option<usize>,
    component: Option<usize>,
    subcomponent: Option<usize>,
}

fn parse_field_path(spec_str: &str) -> Result<FieldPath, String> {
    let bad = || {
        format!(
            "{spec_str:?} is not a field path - use SEG-n[.component[.subcomponent]], e.g. PID-5.1 or OBX[2]-5"
        )
    };
    let (head, tail) = spec_str.split_once('-').ok_or_else(bad)?;
    let (segment, occurrence) = match head.split_once('[') {
        Some((name, rest)) => {
            let n: usize = rest.trim_end_matches(']').parse().map_err(|_| bad())?;
            (name.to_string(), n.max(1))
        }
        None => (head.to_string(), 1),
    };
    if segment.chars().count() != 3 {
        return Err(bad());
    }
    let mut parts = tail.split('.');
    let field_part = parts.next().ok_or_else(bad)?;
    let (field_str, repetition) = match field_part.split_once('[') {
        Some((f, rest)) => (
            f,
            Some(
                rest.trim_end_matches(']')
                    .parse::<usize>()
                    .map_err(|_| bad())?,
            ),
        ),
        None => (field_part, None),
    };
    let field: usize = field_str.parse().map_err(|_| bad())?;
    let component = parts
        .next()
        .map(str::parse::<usize>)
        .transpose()
        .map_err(|_| bad())?;
    let subcomponent = parts
        .next()
        .map(str::parse::<usize>)
        .transpose()
        .map_err(|_| bad())?;
    Ok(FieldPath {
        segment: segment.to_uppercase(),
        occurrence,
        field,
        repetition,
        component,
        subcomponent,
    })
}

fn query_field(
    files: &[ScannedFile],
    path_str: &str,
    o: &render::Options,
) -> Result<ExitCode, String> {
    let path = parse_field_path(path_str)?;
    let mut found = false;
    let multi = files.len() > 1 || files.iter().map(|f| f.message_count).sum::<usize>() > 1;
    let mut out = Output::stdout();
    for file in files {
        let mut i = 0usize;
        for raw in file.selected() {
            let Ok(msg) = parser::parse_message(raw) else {
                continue;
            };
            i += 1;
            let seg = msg
                .segments
                .iter()
                .filter(|s| s.name == path.segment)
                .nth(path.occurrence - 1);
            let Some(seg) = seg else { continue };
            let Some(field) = seg.field(path.field) else {
                continue;
            };
            let reps: Vec<usize> = match path.repetition {
                Some(r) => vec![r],
                None => (1..=field.reps.len()).collect(),
            };
            for r in reps {
                let rep = field.rep(r);
                let value = match (path.component, path.subcomponent) {
                    (None, _) => rep.raw(&msg.sep),
                    (Some(c), None) => rep.comp_text(c, &msg.sep),
                    (Some(c), Some(s)) => rep.comp(c).sub(s).to_string(),
                };
                let value = parser::unescape(&value, &msg.sep);
                found = true;
                let line = if multi {
                    o.paint.dim(&format!("{}#{}", file.label, i)) + "\t" + &value
                } else {
                    value
                };
                writeln!(out, "{line}").map_err(|e| format!("stdout: {e}"))?;
            }
        }
    }
    out.finish().map_err(|e| format!("stdout: {e}"))?;
    if !found {
        return Ok(ExitCode::from(1));
    }
    Ok(ExitCode::SUCCESS)
}

// ------------------------------------------------------------------ json output

/// Serialises the report without ever holding more than one message's JSON.
/// Messages are parsed and validated as the serialiser walks them; the tallies
/// the trailing `status` needs are complete by the time it is written, because
/// `serde_json` emits map keys in alphabetical order and `status` sorts after
/// `files`.
struct Document<'a> {
    files: &'a [ScannedFile],
    parse_failures: usize,
    worst: Cell<Option<Severity>>,
}

impl Document<'_> {
    const fn status(&self) -> &'static str {
        match self.worst.get() {
            Some(Severity::Error) => "error",
            Some(Severity::Warning) => "warning",
            _ if self.parse_failures > 0 => "error",
            _ => "ok",
        }
    }
}

impl Serialize for Document<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("files", &Files(self))?;
        map.serialize_entry("status", self.status())?;
        map.serialize_entry("tool", "hl7probe")?;
        map.serialize_entry("version", env!("CARGO_PKG_VERSION"))?;
        map.end()
    }
}

struct Files<'a>(&'a Document<'a>);

impl Serialize for Files<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.files.len()))?;
        for file in self.0.files {
            seq.serialize_element(&FileEntry { doc: self.0, file })?;
        }
        seq.end()
    }
}

struct FileEntry<'a> {
    doc: &'a Document<'a>,
    file: &'a ScannedFile,
}

impl Serialize for FileEntry<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("file", &self.file.label)?;
        map.serialize_entry(
            "messages",
            &Messages {
                doc: self.doc,
                file: self.file,
            },
        )?;
        map.serialize_entry("notes", &self.file.notes)?;
        map.serialize_entry("parse_errors", &self.file.errors)?;
        map.end()
    }
}

struct Messages<'a> {
    doc: &'a Document<'a>,
    file: &'a ScannedFile,
}

impl Serialize for Messages<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.file.message_count))?;
        for raw in self.file.selected() {
            let Ok(msg) = parser::parse_message(raw) else {
                continue; // Counted as a parse error by the scan.
            };
            let report = validate::validate(&msg);
            self.doc.worst.set(self.doc.worst.get().max(report.worst()));
            seq.serialize_element(&message_json(&msg, &report))?;
        }
        seq.end()
    }
}

fn emit_json(files: &[ScannedFile], strict: bool) -> Result<ExitCode, String> {
    let doc = Document {
        files,
        parse_failures: files.iter().map(|f| f.errors.len()).sum(),
        worst: Cell::new(None),
    };
    let mut out = Output::stdout();
    let mut serializer = serde_json::Serializer::pretty(&mut out);
    doc.serialize(&mut serializer)
        .map_err(|e| format!("stdout: {e}"))?;
    writeln!(out).map_err(|e| format!("stdout: {e}"))?;
    out.finish().map_err(|e| format!("stdout: {e}"))?;
    Ok(exit_code(doc.worst.get(), doc.parse_failures, strict))
}

fn message_json(msg: &Message, report: &Report) -> serde_json::Value {
    let sep = &msg.sep;
    let (code, trigger, structure) = msg.message_type();
    let segments: Vec<serde_json::Value> = msg
        .segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            let fields: Vec<serde_json::Value> = (1..=seg.last_populated())
                .filter(|s| seg.has(*s))
                .filter_map(|s| {
                    let field = seg.field(s)?;
                    Some(serde_json::json!({
                        "seq": s,
                        "name": spec::field_label(&seg.name, s),
                        "value": parser::unescape(&field.raw(sep), sep),
                        "repetitions": field.reps.iter().map(|r| {
                            r.comps.iter().map(|c| c.subs.clone()).collect::<Vec<_>>()
                        }).collect::<Vec<_>>(),
                    }))
                })
                .collect();
            serde_json::json!({
                "name": seg.name,
                "description": spec::segment_desc(&seg.name),
                "occurrence": seg.occurrence,
                "line": seg.line,
                "severity": report.segment_severity(i, true).map(Severity::label),
                "fields": fields,
            })
        })
        .collect();

    let findings: Vec<serde_json::Value> = report
        .findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "severity": f.severity.label(),
                "category": f.category.title(),
                "location": f.location,
                "line": f.line,
                "summary": f.summary,
                "detail": f.detail,
            })
        })
        .collect();

    serde_json::json!({
        "version": msg.version(),
        "type": msg.type_label(),
        "message_code": code,
        "trigger_event": trigger,
        "structure": structure,
        "description": report.structure.map(|s| s.desc),
        "control_id": msg.control_id(),
        "line": msg.start_line,
        "segments": segments,
        "findings": findings,
        "summary": {
            "errors": report.errors(),
            "warnings": report.warnings(),
            "notes": report.count(Severity::Info),
        },
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "panicking is the failure mode a test wants"
    )]

    use super::{decode, exit_code, label_for, parse_field_path};
    use crate::validate::Severity;
    use std::path::Path;
    use std::process::ExitCode;

    /// `ExitCode` is opaque, so compare the debug form the runtime prints.
    fn code(c: ExitCode) -> String {
        format!("{c:?}")
    }

    #[test]
    fn exit_status_follows_the_documented_contract() {
        let clean = code(ExitCode::SUCCESS);
        let failure = code(ExitCode::from(1));
        let unreadable = code(ExitCode::from(2));

        assert_eq!(code(exit_code(None, 0, false)), clean);
        assert_eq!(code(exit_code(Some(Severity::Error), 0, false)), failure);
        assert_eq!(code(exit_code(Some(Severity::Warning), 0, false)), clean);
        assert_eq!(code(exit_code(Some(Severity::Info), 0, true)), clean);
        // --strict promotes warnings, but never notes.
        assert_eq!(code(exit_code(Some(Severity::Warning), 0, true)), failure);
        // An unreadable message outranks any finding severity.
        assert_eq!(code(exit_code(None, 1, false)), unreadable);
        assert_eq!(code(exit_code(Some(Severity::Error), 1, true)), unreadable);
    }

    #[test]
    fn field_paths_decompose_into_their_parts() {
        let p = parse_field_path("PID-5.1").unwrap();
        assert_eq!((p.segment.as_str(), p.occurrence, p.field), ("PID", 1, 5));
        assert_eq!(
            (p.repetition, p.component, p.subcomponent),
            (None, Some(1), None)
        );

        let p = parse_field_path("obx[2]-5").unwrap();
        assert_eq!((p.segment.as_str(), p.occurrence, p.field), ("OBX", 2, 5));

        let p = parse_field_path("PID-3[2].4.1").unwrap();
        assert_eq!(
            (p.field, p.repetition, p.component, p.subcomponent),
            (3, Some(2), Some(4), Some(1))
        );

        // Occurrences are 1-based; [0] means the first one, not "none".
        assert_eq!(parse_field_path("PID[0]-5").unwrap().occurrence, 1);
    }

    #[test]
    fn malformed_field_paths_are_rejected() {
        for bad in [
            "PID", "-5", "PID-", "PIDD-1", "PI-1", "PID-x", "PID-5.x", "PID[x]-5", "", "-",
        ] {
            assert!(parse_field_path(bad).is_err(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn non_utf8_input_falls_back_to_latin_1() {
        assert_eq!(decode(b"MSH|^~\\&|".to_vec()), "MSH|^~\\&|");
        // 0xE9 is a bare latin-1 "e-acute": invalid UTF-8, still readable.
        assert_eq!(decode(vec![b'A', 0xE9, b'B']), "A\u{e9}B");
    }

    #[test]
    fn labels_prefer_the_file_name() {
        assert_eq!(label_for(Path::new("/tmp/adt_a01.hl7")), "adt_a01.hl7");
        assert_eq!(label_for(Path::new("adt.hl7")), "adt.hl7");
        assert_eq!(label_for(Path::new("..")), "..");
    }
}
