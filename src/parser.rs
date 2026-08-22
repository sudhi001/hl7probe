//! HL7 v2 lexical parser: MLLP/batch stripping, segment/field/component/subcomponent
//! decomposition and escape-sequence handling.

use std::fmt::Write as _;

use std::fmt;

/// The five delimiters an HL7 v2 message declares in MSH-1 and MSH-2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Separators {
    pub field: char,
    pub component: char,
    pub repetition: char,
    pub escape: char,
    pub subcomponent: char,
}

impl Default for Separators {
    fn default() -> Self {
        Self {
            field: '|',
            component: '^',
            repetition: '~',
            escape: '\\',
            subcomponent: '&',
        }
    }
}

impl Separators {
    /// Reads MSH-1 (the character right after `MSH`) and MSH-2 (the encoding
    /// characters up to the next field separator).
    fn from_msh(line: &str) -> Result<Self, ParseError> {
        let chars: Vec<char> = line.chars().collect();
        if chars.len() < 4 {
            return Err(ParseError::new(
                0,
                "MSH segment is truncated before the field separator",
            ));
        }
        let field = chars[3];
        if field.is_alphanumeric() || field.is_whitespace() {
            return Err(ParseError::new(
                0,
                format!("MSH-1 field separator {field:?} is not a usable delimiter"),
            ));
        }
        let enc: String = chars[4..].iter().take_while(|c| **c != field).collect();
        let e: Vec<char> = enc.chars().collect();
        let mut sep = Self {
            field,
            ..Default::default()
        };
        if !e.is_empty() {
            sep.component = e[0];
        }
        if e.len() > 1 {
            sep.repetition = e[1];
        }
        if e.len() > 2 {
            sep.escape = e[2];
        }
        if e.len() > 3 {
            sep.subcomponent = e[3];
        }
        if e.len() > 4 {
            return Err(ParseError::new(
                0,
                format!(
                    "MSH-2 declares {} encoding characters, expected at most 4",
                    e.len()
                ),
            ));
        }
        let all = [
            sep.field,
            sep.component,
            sep.repetition,
            sep.escape,
            sep.subcomponent,
        ];
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                if all[i] == all[j] {
                    return Err(ParseError::new(
                        0,
                        format!("delimiter {:?} is declared twice in MSH-1/MSH-2", all[i]),
                    ));
                }
            }
        }
        Ok(sep)
    }
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl ParseError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: {}", self.line, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

/// A single component, itself made of `&`-delimited subcomponents.
#[derive(Debug, Clone)]
pub struct Component {
    pub subs: Vec<String>,
}

impl Component {
    pub fn sub(&self, seq: usize) -> &str {
        self.subs
            .get(seq.wrapping_sub(1))
            .map_or("", String::as_str)
    }
    pub fn is_empty(&self) -> bool {
        self.subs.iter().all(String::is_empty)
    }
    fn raw(&self, sep: &Separators) -> String {
        self.subs.join(&sep.subcomponent.to_string())
    }
}

/// One repetition of a field (`~`-delimited at the field level).
#[derive(Debug, Clone)]
pub struct Repetition {
    pub comps: Vec<Component>,
}

impl Repetition {
    pub fn comp(&self, seq: usize) -> &Component {
        const EMPTY: &Component = &Component { subs: Vec::new() };
        self.comps.get(seq.wrapping_sub(1)).unwrap_or(EMPTY)
    }
    /// Component `seq` rendered as text (subcomponents rejoined).
    pub fn comp_text(&self, seq: usize, sep: &Separators) -> String {
        self.comp(seq).raw(sep)
    }
    pub fn is_empty(&self) -> bool {
        self.comps.iter().all(Component::is_empty)
    }
    pub fn raw(&self, sep: &Separators) -> String {
        self.comps
            .iter()
            .map(|c| c.raw(sep))
            .collect::<Vec<_>>()
            .join(&sep.component.to_string())
    }
    /// Number of components actually carrying data.
    pub fn filled_comps(&self) -> usize {
        self.comps
            .iter()
            .rposition(|c| !c.is_empty())
            .map_or(0, |i| i + 1)
    }
}

/// A field: one or more repetitions.
#[derive(Debug, Clone)]
pub struct Field {
    pub reps: Vec<Repetition>,
    /// Set for MSH-1/MSH-2, whose values are the delimiters themselves and must
    /// never be re-split.
    literal: Option<String>,
}

impl Field {
    fn literal(value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            reps: vec![Repetition {
                comps: vec![Component {
                    subs: vec![value.clone()],
                }],
            }],
            literal: Some(value),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.reps.iter().all(Repetition::is_empty)
    }

    /// HL7 explicit null: the two-character value `""` means "delete this value".
    pub fn is_null(&self) -> bool {
        self.reps.len() == 1
            && self.reps[0].comps.len() == 1
            && self.reps[0].comp(1).sub(1) == "\"\""
    }

    pub fn rep(&self, seq: usize) -> &Repetition {
        const EMPTY: &Repetition = &Repetition { comps: Vec::new() };
        self.reps.get(seq.wrapping_sub(1)).unwrap_or(EMPTY)
    }

    /// First repetition, component `seq`, as text.
    pub fn comp(&self, seq: usize, sep: &Separators) -> String {
        self.rep(1).comp_text(seq, sep)
    }

    /// Whole field as it appeared on the wire (repetitions included).
    pub fn raw(&self, sep: &Separators) -> String {
        if let Some(lit) = &self.literal {
            return lit.clone();
        }
        self.reps
            .iter()
            .map(|r| r.raw(sep))
            .collect::<Vec<_>>()
            .join(&sep.repetition.to_string())
    }
}

/// One segment line.
#[derive(Debug, Clone)]
pub struct Segment {
    pub name: String,
    /// 1-based line number in the source file, for error reporting.
    pub line: usize,
    /// 1-based occurrence among segments with the same name.
    pub occurrence: usize,
    /// Index 0 holds field 1.
    pub fields: Vec<Field>,
    pub raw: String,
}

impl Segment {
    pub fn field(&self, seq: usize) -> Option<&Field> {
        self.fields.get(seq.wrapping_sub(1))
    }

    /// True when field `seq` exists and carries data.
    pub fn has(&self, seq: usize) -> bool {
        self.field(seq).is_some_and(|f| !f.is_empty())
    }

    /// Field `seq` as raw text, or `""` when absent.
    pub fn text(&self, seq: usize, sep: &Separators) -> String {
        self.field(seq).map(|f| f.raw(sep)).unwrap_or_default()
    }

    /// First repetition, component `c`, of field `seq`.
    pub fn comp(&self, seq: usize, c: usize, sep: &Separators) -> String {
        self.field(seq).map(|f| f.comp(c, sep)).unwrap_or_default()
    }

    /// Highest field number carrying data.
    pub fn last_populated(&self) -> usize {
        self.fields
            .iter()
            .rposition(|f| !f.is_empty())
            .map_or(0, |i| i + 1)
    }

    /// Z-segments are site-defined and exempt from dictionary checks.
    pub fn is_custom(&self) -> bool {
        self.name.starts_with('Z')
    }

    fn parse(name: &str, raw: &str, line: usize, sep: &Separators) -> Self {
        let parts: Vec<&str> = raw.split(sep.field).collect();
        let mut fields: Vec<Field> = Vec::new();
        // MSH is positionally special: MSH-1 *is* the field separator, so the
        // first split part after the name is MSH-2, not MSH-1.
        let rest = if name == "MSH" {
            fields.push(Field::literal(sep.field.to_string()));
            fields.push(Field::literal(parts.get(1).copied().unwrap_or("")));
            &parts[2.min(parts.len())..]
        } else {
            &parts[1.min(parts.len())..]
        };
        for part in rest {
            fields.push(parse_field(part, sep));
        }
        Self {
            name: name.to_string(),
            line,
            occurrence: 1,
            fields,
            raw: raw.to_string(),
        }
    }
}

fn parse_field(s: &str, sep: &Separators) -> Field {
    let reps = s
        .split(sep.repetition)
        .map(|rep| Repetition {
            comps: rep
                .split(sep.component)
                .map(|c| Component {
                    subs: c.split(sep.subcomponent).map(ToString::to_string).collect(),
                })
                .collect(),
        })
        .collect();
    Field {
        reps,
        literal: None,
    }
}

/// A fully decomposed HL7 message.
#[derive(Debug, Clone)]
pub struct Message {
    pub sep: Separators,
    pub segments: Vec<Segment>,
    /// 1-based line where this message's MSH was found.
    pub start_line: usize,
    /// Non-fatal observations made while tokenising (stray bytes, batch wrappers).
    pub notes: Vec<String>,
}

impl Message {
    pub fn msh(&self) -> &Segment {
        &self.segments[0]
    }

    /// MSH-12.1, e.g. `2.5.1`.
    pub fn version(&self) -> String {
        self.msh().comp(12, 1, &self.sep)
    }

    /// (message code, trigger event, structure) from MSH-9.
    pub fn message_type(&self) -> (String, String, String) {
        let f = self.msh();
        (
            f.comp(9, 1, &self.sep),
            f.comp(9, 2, &self.sep),
            f.comp(9, 3, &self.sep),
        )
    }

    /// `ADT^A01`, or just `ADT` when no trigger event is present.
    pub fn type_label(&self) -> String {
        let (code, trigger, _) = self.message_type();
        match (code.is_empty(), trigger.is_empty()) {
            (true, _) => "(no MSH-9)".to_string(),
            (false, true) => code,
            (false, false) => format!("{code}^{trigger}"),
        }
    }

    pub fn control_id(&self) -> String {
        self.msh().comp(10, 1, &self.sep)
    }

    pub fn find(&self, name: &str) -> Vec<&Segment> {
        self.segments.iter().filter(|s| s.name == name).collect()
    }

    pub fn first(&self, name: &str) -> Option<&Segment> {
        self.segments.iter().find(|s| s.name == name)
    }
}

/// One message's worth of source lines, still unparsed.
/// Where a message sits in the file, rather than a copy of it or an index of
/// its lines. Splitting the lines again when the message is parsed costs the
/// same walk either way, and it keeps a whole batch down to a few bytes per
/// message instead of twenty-four per line.
pub struct RawMessage<'a> {
    pub start_line: usize,
    text: &'a str,
    pub notes: Vec<String>,
}

impl<'a> RawMessage<'a> {
    /// The segment lines this message is made of, cleaned of framing bytes and
    /// numbered as they are in the file. Blank lines and batch wrappers are
    /// skipped here exactly as `split_messages` skipped them.
    pub fn lines(&self) -> impl Iterator<Item = (usize, &'a str)> + '_ {
        lines(self.text)
            .enumerate()
            .filter_map(move |(offset, (_, line))| {
                let cleaned = clean(line);
                if cleaned.is_empty() || is_batch_wrapper(head(cleaned)) {
                    return None;
                }
                Some((self.start_line + offset, cleaned))
            })
    }

    /// The MSH line, which every message starts with and which decides on its
    /// own whether the message can be read at all.
    fn first_line(&self) -> (usize, &'a str) {
        self.lines().next().unwrap_or((self.start_line, self.text))
    }
}

/// Splits on CR, LF or CRLF, counting CRLF as one break so line numbers match
/// what an editor shows, and reports where each line starts. Iterating beats
/// normalising the whole file into a new `String` first, which cost two full
/// copies of the input.
fn lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut offset = 0usize;
    let mut rest = Some(text);
    std::iter::from_fn(move || {
        let current = rest?;
        let start = offset;
        match current.find(['\r', '\n']) {
            None => {
                rest = None;
                Some((start, current))
            }
            Some(at) => {
                let (line, tail) = current.split_at(at);
                let skip = usize::from(tail.starts_with("\r\n")) + 1;
                offset += at + skip;
                rest = Some(&tail[skip..]);
                Some((start, line))
            }
        }
    })
}

/// Strips MLLP framing bytes and surrounding whitespace from a line.
fn clean(line: &str) -> &str {
    line.trim_matches(|c: char| {
        c == '\u{0b}' || c == '\u{1c}' || c == '\u{1d}' || c == '\0' || c.is_whitespace()
    })
}

fn is_batch_wrapper(head: &str) -> bool {
    matches!(head, "FHS" | "BHS" | "BTS" | "FTS")
}

/// The first three characters of a line, which is where a segment name lives.
fn head(text: &str) -> &str {
    let end = text.char_indices().nth(3).map_or(text.len(), |(i, _)| i);
    &text[..end]
}

/// Splits a file into messages, tolerating CR/LF/CRLF endings, MLLP framing
/// bytes and HL7 batch (FHS/BHS/BTS/FTS) wrappers.
pub fn split_messages(raw: &str) -> (Vec<RawMessage<'_>>, Vec<String>) {
    let mut messages: Vec<RawMessage<'_>> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut pending_notes: Vec<String> = Vec::new();
    let mut stray_reported = false;
    // Byte range of the message being accumulated, so it can be sliced out of
    // `raw` once the next MSH (or the end of the file) closes it.
    let mut open: Option<(usize, usize)> = None;

    for (idx, (offset, line)) in lines(raw).enumerate() {
        let lineno = idx + 1;
        let cleaned = clean(line);
        if cleaned.is_empty() {
            continue;
        }
        let head = head(cleaned);
        if is_batch_wrapper(head) {
            pending_notes.push(format!("line {lineno}: batch wrapper {head} skipped"));
            continue;
        }
        let line_end = offset + line.len();
        if head == "MSH" {
            if let Some((start, end)) = open.replace((offset, line_end)) {
                messages
                    .last_mut()
                    .expect("an open range has a message")
                    .text = &raw[start..end];
            }
            messages.push(RawMessage {
                start_line: lineno,
                text: &raw[offset..line_end],
                notes: std::mem::take(&mut pending_notes),
            });
        } else if let Some((_, end)) = open.as_mut() {
            *end = line_end;
        } else if !stray_reported {
            stray_reported = true;
            warnings.push(format!(
                "line {lineno}: content before the first MSH segment was ignored"
            ));
        }
    }
    if let Some((start, end)) = open {
        messages
            .last_mut()
            .expect("an open range has a message")
            .text = &raw[start..end];
    }
    (messages, warnings)
}

/// The three-character name a line starts with, when it is a usable segment
/// name. Shared so the whole-message check and the segment loop cannot drift.
fn segment_name(text: &str) -> Option<String> {
    let name: String = text.chars().take(3).collect();
    let usable = name.chars().count() == 3
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && name.starts_with(|c: char| c.is_ascii_uppercase());
    usable.then_some(name)
}

impl RawMessage<'_> {
    /// The delimiters this message declares, or the reason it cannot be read.
    /// Both of `parse_message`'s failure modes are settled by the first line,
    /// so a batch can be checked for unreadable messages without building a
    /// single tree.
    pub fn separators(&self) -> Result<Separators, ParseError> {
        let (lineno, text) = self.first_line();
        let sep = Separators::from_msh(text).map_err(|e| ParseError::new(lineno, e.message))?;
        if segment_name(text).as_deref() != Some("MSH") {
            return Err(ParseError::new(
                lineno,
                "message does not begin with a parsable MSH segment",
            ));
        }
        Ok(sep)
    }
}

pub fn parse_message(raw: &RawMessage<'_>) -> Result<Message, ParseError> {
    let sep = raw.separators()?;

    let mut segments: Vec<Segment> = Vec::new();
    let mut notes = raw.notes.clone();
    let mut counts: Vec<(String, usize)> = Vec::new();

    for (lineno, text) in raw.lines() {
        let Some(name) = segment_name(text) else {
            notes.push(format!(
                "line {}: skipped unrecognisable segment starting {:?}",
                lineno,
                text.chars().take(8).collect::<String>()
            ));
            continue;
        };
        if text.chars().nth(3) != Some(sep.field) {
            notes.push(format!(
                "line {lineno}: segment {name} has no field separator after the name"
            ));
        }
        let mut seg = Segment::parse(&name, text, lineno, &sep);
        let entry = counts.iter_mut().find(|(n, _)| n == &name);
        seg.occurrence = if let Some((_, c)) = entry {
            *c += 1;
            *c
        } else {
            counts.push((name.clone(), 1));
            1
        };
        segments.push(seg);
    }

    Ok(Message {
        sep,
        segments,
        start_line: raw.start_line,
        notes,
    })
}

/// Resolves HL7 escape sequences for human-readable display.
pub fn unescape(s: &str, sep: &Separators) -> String {
    if !s.contains(sep.escape) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != sep.escape {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let end = chars[i + 1..]
            .iter()
            .position(|c| *c == sep.escape)
            .map(|p| i + 1 + p);
        let Some(end) = end else {
            out.push(chars[i]);
            i += 1;
            continue;
        };
        let code: String = chars[i + 1..end].iter().collect();
        match code.as_str() {
            "F" => out.push(sep.field),
            "S" => out.push(sep.component),
            "T" => out.push(sep.subcomponent),
            "R" => out.push(sep.repetition),
            ".br" | ".sp" => out.push('\n'),
            // \E\ is the escape character itself; \\ is the same thing written bare.
            "E" | "" => out.push(sep.escape),
            other if other.starts_with('X') => {
                let hex = &other[1..];
                // Collecting into `Option<Vec<_>>` stops at the first bad pair,
                // so a malformed \Xnn\ falls through to the literal branch.
                let decoded = (!hex.is_empty() && hex.len() % 2 == 0)
                    .then(|| {
                        hex.as_bytes()
                            .chunks(2)
                            .map(|pair| {
                                u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()
                            })
                            .collect::<Option<Vec<u8>>>()
                    })
                    .flatten();
                match decoded {
                    Some(bytes) => out.push_str(&String::from_utf8_lossy(&bytes)),
                    None => {
                        let _ = write!(out, "{}{}{}", sep.escape, other, sep.escape);
                    }
                }
            }
            // Highlighting and site-defined escapes carry no display text.
            other if other.starts_with('H') || other.starts_with('N') || other.starts_with('Z') => {
            }
            other => {
                let _ = write!(out, "{}{}{}", sep.escape, other, sep.escape);
            }
        }
        i = end + 1;
    }
    out
}

#[cfg(test)]
pub fn parse_str(text: &str) -> Message {
    let (raws, _) = split_messages(text);
    parse_message(&raws[0]).expect("fixture should parse")
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "panicking is the failure mode a test wants"
    )]
    use super::*;

    const ADT: &str = "MSH|^~\\&|HIS|MERCY|LIS|LAB|20240115143200||ADT^A01^ADT_A01|MSG1|P|2.5.1\r\
PID|1||123456^^^MERCY^MR~999^^^SSA^SS||Smith^John^A||19850312|M\r\
PV1|1|I|ER^101^A&Bay 2^MERCY\r";

    #[test]
    fn reads_default_delimiters() {
        let m = parse_str(ADT);
        assert_eq!(m.sep, Separators::default());
        assert_eq!(m.segments.len(), 3);
    }

    #[test]
    fn honours_custom_delimiters() {
        let m = parse_str("MSH#@~\\&#A#B#C#D#20240101120000##ADT@A01#1#P#2.5.1\r");
        assert_eq!(m.sep.field, '#');
        assert_eq!(m.sep.component, '@');
        assert_eq!(m.type_label(), "ADT^A01");
    }

    #[test]
    fn msh_field_numbering_is_offset_by_the_separator() {
        let m = parse_str(ADT);
        let msh = m.msh();
        assert_eq!(msh.text(1, &m.sep), "|");
        assert_eq!(msh.text(2, &m.sep), "^~\\&");
        assert_eq!(msh.text(3, &m.sep), "HIS");
        assert_eq!(msh.comp(9, 2, &m.sep), "A01");
        assert_eq!(m.version(), "2.5.1");
        assert_eq!(m.control_id(), "MSG1");
    }

    #[test]
    fn splits_repetitions_components_and_subcomponents() {
        let m = parse_str(ADT);
        let pid = m.first("PID").unwrap();
        let ids = pid.field(3).unwrap();
        assert_eq!(ids.reps.len(), 2);
        assert_eq!(ids.rep(2).comp_text(1, &m.sep), "999");
        assert_eq!(ids.rep(1).comp_text(5, &m.sep), "MR");

        let pv1 = m.first("PV1").unwrap();
        let location = pv1.field(3).unwrap().rep(1);
        assert_eq!(location.comp(3).sub(1), "A");
        assert_eq!(location.comp(3).sub(2), "Bay 2");
    }

    #[test]
    fn tracks_segment_occurrence_and_line() {
        let m = parse_str("MSH|^~\\&|A|B|C|D|20240101120000||ORU^R01|1|P|2.5.1\rOBX|1\rOBX|2\r");
        let obx = m.find("OBX");
        assert_eq!(obx.len(), 2);
        assert_eq!(obx[1].occurrence, 2);
        assert_eq!(obx[1].line, 3);
    }

    #[test]
    fn accepts_lf_crlf_and_mllp_framing() {
        for text in [
            "MSH|^~\\&|A|B|C|D|20240101120000||ACK|1|P|2.5.1\nMSA|AA|1\n",
            "MSH|^~\\&|A|B|C|D|20240101120000||ACK|1|P|2.5.1\r\nMSA|AA|1\r\n",
            "\u{b}MSH|^~\\&|A|B|C|D|20240101120000||ACK|1|P|2.5.1\rMSA|AA|1\r\u{1c}\r",
        ] {
            let m = parse_str(text);
            assert_eq!(m.segments.len(), 2, "{text:?}");
            assert_eq!(m.segments[1].name, "MSA");
        }
    }

    #[test]
    fn skips_batch_wrappers_and_splits_messages() {
        let text = "FHS|^~\\&\rBHS|^~\\&\r\
MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|1|P|2.5.1\rPID|1\r\
MSH|^~\\&|A|B|C|D|20240101130000||ADT^A03|2|P|2.5.1\rPID|1\rBTS|2\rFTS|1\r";
        let (raws, warnings) = split_messages(text);
        assert_eq!(raws.len(), 2);
        assert!(warnings.is_empty());
        let first = parse_message(&raws[0]).unwrap();
        assert_eq!(first.segments.len(), 2);
        assert_eq!(first.notes.len(), 2, "batch wrappers should be noted");
        assert_eq!(parse_message(&raws[1]).unwrap().control_id(), "2");
    }

    #[test]
    fn rejects_input_without_msh() {
        let (raws, warnings) = split_messages("PID|1||123\r");
        assert!(raws.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn rejects_duplicate_delimiters() {
        let (raws, _) = split_messages("MSH|^~\\^|A|B|C|D|20240101120000||ACK|1|P|2.5.1\r");
        assert!(parse_message(&raws[0]).is_err());
    }

    #[test]
    fn detects_explicit_null() {
        let m = parse_str("MSH|^~\\&|A|B|C|D|20240101120000||ADT^A08|1|P|2.5.1\rPID|1||\"\"\r");
        assert!(m.first("PID").unwrap().field(3).unwrap().is_null());
    }

    #[test]
    fn resolves_escape_sequences() {
        let sep = Separators::default();
        assert_eq!(unescape("Smith \\T\\ Sons", &sep), "Smith & Sons");
        assert_eq!(unescape("100\\S\\200", &sep), "100^200");
        assert_eq!(unescape("a\\F\\b", &sep), "a|b");
        assert_eq!(unescape("line1\\.br\\line2", &sep), "line1\nline2");
        assert_eq!(unescape("\\X0A\\", &sep), "\n");
        assert_eq!(unescape("50\\E\\50", &sep), "50\\50");
        // Unknown escapes survive untouched rather than eating the text.
        assert_eq!(unescape("a\\Q9\\b", &sep), "a\\Q9\\b");
    }

    #[test]
    fn last_populated_ignores_trailing_empties() {
        let m = parse_str(
            "MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|1|P|2.5.1\rEVN|A01|20240101120000||||\r",
        );
        assert_eq!(m.first("EVN").unwrap().last_populated(), 2);
    }
}
