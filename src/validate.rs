//! Validation passes: structure against the abstract message grammar, field
//! usage, data types, code tables and cross-field consistency.

use crate::datetime::{self, Precision};
use crate::parser::{Message, Repetition, Segment};
use crate::spec::{self, Use};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Error => "\u{2717}",   // ✗
            Self::Warning => "\u{26a0}", // ⚠
            Self::Info => "\u{2139}",    // ℹ
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Structure,
    Required,
    DataType,
    CodeTable,
    Consistency,
}

impl Category {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Structure => "Structure",
            Self::Required => "Required fields",
            Self::DataType => "Data types",
            Self::CodeTable => "Code tables",
            Self::Consistency => "Consistency",
        }
    }
    pub const ALL: [Self; 5] = [
        Self::Structure,
        Self::Required,
        Self::DataType,
        Self::CodeTable,
        Self::Consistency,
    ];
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub category: Category,
    /// `PID-11`, `PV1` or an empty string for message-level findings.
    pub location: String,
    pub line: usize,
    /// Index into `Message::segments`, when the finding belongs to one.
    pub segment_index: Option<usize>,
    pub summary: String,
    pub detail: String,
}

impl Finding {
    fn new(
        severity: Severity,
        category: Category,
        location: impl Into<String>,
        summary: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            category,
            location: location.into(),
            line: 0,
            segment_index: None,
            summary: summary.into(),
            detail: detail.into(),
        }
    }
    const fn at(mut self, seg: &Segment<'_>, index: usize) -> Self {
        self.line = seg.line;
        self.segment_index = Some(index);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Report {
    pub findings: Vec<Finding>,
    /// Structure name matched from MSH-9, e.g. `ADT^A01 Admit / Visit Notification`.
    pub structure: Option<&'static spec::MessageSpec>,
}

impl Report {
    pub fn count(&self, sev: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == sev).count()
    }
    pub fn errors(&self) -> usize {
        self.count(Severity::Error)
    }
    pub fn warnings(&self) -> usize {
        self.count(Severity::Warning)
    }
    pub fn worst(&self) -> Option<Severity> {
        self.findings.iter().map(|f| f.severity).max()
    }
    /// Worst severity recorded against a given segment occurrence. Info-level
    /// notes are excluded unless `include_info` is set, so the segment list
    /// agrees with the findings the reader is actually shown.
    pub fn segment_severity(&self, index: usize, include_info: bool) -> Option<Severity> {
        self.findings
            .iter()
            .filter(|f| f.segment_index == Some(index))
            .filter(|f| include_info || f.severity != Severity::Info)
            .map(|f| f.severity)
            .max()
    }
    pub fn for_segment(&self, index: usize) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.segment_index == Some(index))
            .collect()
    }
}

pub fn validate(msg: &Message<'_>) -> Report {
    let mut findings: Vec<Finding> = Vec::new();
    for rule in RULES {
        rule.check(msg, &mut findings);
    }
    // Most severe first, then message order, then field order within a segment
    // (numeric, so PID-8 sorts before PID-11).
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then(a.line.cmp(&b.line))
            .then(field_seq(&a.location).cmp(&field_seq(&b.location)))
            .then(a.location.cmp(&b.location))
    });
    let (code, trigger, _) = msg.message_type();
    Report {
        findings,
        structure: spec::message_spec(code, trigger),
    }
}

/// The parser guarantees a message starts with MSH, so its findings always
/// belong to the first segment.
const MSH_INDEX: usize = 0;

/// One independent validation concern.
///
/// Rules never see each other: a new check is a new implementation added to
/// `RULES`, not an edit to an existing pass.
trait Rule {
    fn check(&self, msg: &Message<'_>, out: &mut Vec<Finding>);
}

/// Every rule applied to a message, in the order their findings are produced.
const RULES: &[&dyn Rule] = &[
    &StructureRule,
    &FieldUsageRule,
    &ValueTypeRule,
    &CodeTableRule,
    &MessageHeaderRule,
    &EventRule,
    &PatientRule,
    &VisitRule,
    &ObservationRule,
    &SetIdRule,
];

/// One populated repetition of a field the dictionary defines, which is the
/// unit both value-level rules work on.
struct FieldRepetition<'a> {
    seg: &'a Segment<'a>,
    seg_index: usize,
    spec: &'a spec::FieldSpec,
    rep: Repetition<'a>,
    /// `PID-3`, or `PID-3[2]` when the field repeats.
    location: String,
}

/// Walks every populated repetition of every defined field, skipping
/// site-defined segments and HL7 explicit nulls.
fn visit_repetitions(msg: &Message<'_>, mut visit: impl FnMut(FieldRepetition<'_>)) {
    for (seg_index, seg) in msg.segments.iter().enumerate() {
        if seg.is_custom() {
            continue;
        }
        let Some(segment_spec) = spec::segment_spec(seg.name) else {
            continue;
        };
        for field_spec in segment_spec.fields {
            let Some(field) = seg.field(field_spec.seq) else {
                continue;
            };
            if field.is_empty() || field.is_null() {
                continue;
            }
            let location = format!("{}-{}", seg.name, field_spec.seq);
            for (index, rep) in field.reps().enumerate() {
                if rep.is_empty() {
                    continue;
                }
                visit(FieldRepetition {
                    seg,
                    seg_index,
                    spec: field_spec,
                    rep,
                    location: if field.rep_count() > 1 {
                        format!("{}[{}]", location, index + 1)
                    } else {
                        location.clone()
                    },
                });
            }
        }
    }
}

// -------------------------------------------------------------------- structure

/// The message against its abstract structure: required, unexpected and
/// out-of-order segments.
struct StructureRule;

impl Rule for StructureRule {
    fn check(&self, msg: &Message<'_>, out: &mut Vec<Finding>) {
        let (code, trigger, structure_id) = msg.message_type();
        let structure = spec::message_spec(code, trigger);
        check_structure(msg, code, trigger, structure_id, structure, out);
    }
}

fn check_structure(
    msg: &Message<'_>,
    code: &str,
    trigger: &str,
    structure_id: &str,
    structure: Option<&'static spec::MessageSpec>,
    out: &mut Vec<Finding>,
) {
    if code.is_empty() {
        out.push(Finding::new(
            Severity::Error,
            Category::Structure,
            "MSH-9",
            "message type missing",
            "MSH-9.1 carries no message code, so no structure can be validated",
        ));
    } else if spec::code_meaning("0076", code).is_none() {
        out.push(Finding::new(
            Severity::Warning,
            Category::Structure,
            "MSH-9.1",
            format!("unknown message code {code:?}"),
            "not listed in HL7 table 0076 (Message Type)",
        ));
    }

    let Some(structure) = structure else {
        if !code.is_empty() {
            out.push(Finding::new(
                Severity::Info,
                Category::Structure,
                "MSH-9",
                format!("no structure definition for {code}^{trigger}"),
                "segment order and required segments were not checked; field-level checks still ran",
            ));
        }
        return;
    };

    // MSH-9.3 should name the abstract structure, e.g. ADT_A01.
    if !structure_id.is_empty() {
        let expected = format!("{code}_{trigger}");
        if !trigger.is_empty() && structure_id != expected && !structure_id.starts_with(code) {
            out.push(Finding::new(
                Severity::Warning,
                Category::Structure,
                "MSH-9.3",
                format!("structure {structure_id:?} does not match {code}^{trigger}"),
                format!("expected {expected:?}"),
            ));
        }
    }

    for expected in structure.segments.iter().filter(|s| s.required) {
        if msg.first(expected.name).is_none() {
            out.push(Finding::new(
                Severity::Error,
                Category::Structure,
                expected.name,
                "required segment is missing",
                format!(
                    "{} requires {} ({})",
                    structure.id,
                    expected.name,
                    spec::segment_desc(expected.name).unwrap_or("no description")
                ),
            ));
        }
    }

    let mut furthest = 0usize;
    let mut order_reported = false;
    for (i, seg) in msg.segments.iter().enumerate() {
        if seg.is_custom() {
            continue;
        }
        // A segment name can appear at several points in a grammar (NTE in ORU,
        // for example), so match the earliest slot at or after what we have
        // already consumed rather than the first slot overall.
        let slots: Vec<usize> = structure
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.name == seg.name)
            .map(|(i, _)| i)
            .collect();
        if slots.is_empty() {
            let known = spec::segment_desc(seg.name).is_some();
            let (sev, detail) = if known {
                (
                    Severity::Warning,
                    format!("{} is not part of the {} structure", seg.name, structure.id),
                )
            } else {
                (
                    Severity::Warning,
                    format!("{} is not a recognised HL7 segment", seg.name),
                )
            };
            out.push(
                Finding::new(
                    sev,
                    Category::Structure,
                    seg.name,
                    "unexpected segment",
                    detail,
                )
                .at(seg, i),
            );
            continue;
        }
        match slots.iter().copied().find(|p| *p >= furthest) {
            Some(slot) => furthest = slot,
            None if !order_reported => {
                order_reported = true;
                out.push(
                    Finding::new(
                        Severity::Warning,
                        Category::Structure,
                        seg.name,
                        "segment appears out of order",
                        format!(
                            "{} expects {} earlier in the message",
                            structure.id, seg.name
                        ),
                    )
                    .at(seg, i),
                );
            }
            None => {}
        }
    }

    if msg.segments.len() == 1 {
        out.push(Finding::new(
            Severity::Warning,
            Category::Structure,
            "",
            "message contains only an MSH segment",
            "no payload segments were found after the header",
        ));
    }
}

// ----------------------------------------------------------------- field usage

/// Whether each defined field is present as often as the standard expects.
struct FieldUsageRule;

impl Rule for FieldUsageRule {
    fn check(&self, msg: &Message<'_>, out: &mut Vec<Finding>) {
        for (index, seg) in msg.segments.iter().enumerate() {
            check_segment_fields(seg, index, out);
        }
    }
}

/// Whether each value parses as the data type its field declares.
struct ValueTypeRule;

impl Rule for ValueTypeRule {
    fn check(&self, msg: &Message<'_>, out: &mut Vec<Finding>) {
        visit_repetitions(msg, |field| {
            check_datatype(
                field.spec,
                field.rep,
                &field.location,
                field.seg,
                field.seg_index,
                out,
            );
        });
    }
}

/// Whether each coded value appears in the HL7 table its field names.
struct CodeTableRule;

impl Rule for CodeTableRule {
    fn check(&self, msg: &Message<'_>, out: &mut Vec<Finding>) {
        visit_repetitions(msg, |field| {
            let Some(table_id) = field.spec.table else {
                return;
            };
            check_table(
                table_id,
                field.rep.comp_text(1),
                &field.location,
                field.spec.name,
                field.seg,
                field.seg_index,
                out,
            );
        });
    }
}

fn check_segment_fields(seg: &Segment<'_>, index: usize, out: &mut Vec<Finding>) {
    if seg.is_custom() {
        return;
    }
    let Some(sspec) = spec::segment_spec(seg.name) else {
        return;
    };
    let max_defined = sspec.fields.iter().map(|f| f.seq).max().unwrap_or(0);

    for fs in sspec.fields {
        let present = seg.has(fs.seq);
        let loc = format!("{}-{}", seg.name, fs.seq);
        if !present {
            match fs.usage {
                Use::Required => out.push(
                    Finding::new(
                        Severity::Error,
                        Category::Required,
                        loc.clone(),
                        "missing",
                        format!("{} is required by the {} segment", fs.name, seg.name),
                    )
                    .at(seg, index),
                ),
                Use::Recommended => out.push(
                    Finding::new(
                        Severity::Warning,
                        Category::Required,
                        loc.clone(),
                        "missing",
                        format!("{} should be populated when the value is known", fs.name),
                    )
                    .at(seg, index),
                ),
                _ => {}
            }
            continue;
        }

        let Some(field) = seg.field(fs.seq) else {
            continue;
        };
        if field.is_null() {
            continue; // "" is an explicit "delete this value" instruction.
        }
        if fs.usage == Use::Backward {
            out.push(
                Finding::new(
                    Severity::Info,
                    Category::Consistency,
                    loc.clone(),
                    "deprecated field is populated",
                    format!(
                        "{} is deprecated; prefer the current equivalent field",
                        fs.name
                    ),
                )
                .at(seg, index),
            );
        }
        if !fs.repeats && field.rep_count() > 1 {
            out.push(
                Finding::new(
                    Severity::Warning,
                    Category::DataType,
                    loc.clone(),
                    "repeats but is defined as non-repeating",
                    format!(
                        "{} repetitions found in a single-value field",
                        field.rep_count()
                    ),
                )
                .at(seg, index),
            );
        }

        // Value-level checks belong to ValueTypeRule and CodeTableRule; this
        // rule only reports repetitions that carry nothing at all.
        if field.rep_count() > 1 {
            for (ri, rep) in field.reps().enumerate() {
                if rep.is_empty() {
                    out.push(
                        Finding::new(
                            Severity::Warning,
                            Category::DataType,
                            loc.clone(),
                            "has an empty repetition",
                            format!("repetition {} carries no value", ri + 1),
                        )
                        .at(seg, index),
                    );
                }
            }
        }
    }

    // Fields past the end of the dictionary usually mean a mis-aligned message.
    let last = seg.last_populated();
    if last > max_defined {
        let loc = format!("{}-{}", seg.name, last);
        out.push(
            Finding::new(
                Severity::Warning,
                Category::Structure,
                loc,
                format!("data beyond the last defined field ({}-{})", seg.name, max_defined),
                format!(
                    "{} defines fields up to {}-{}; extra fields are usually a delimiter or alignment problem",
                    seg.name, seg.name, max_defined
                ),
            )
            .at(seg, index),
        );
    }
}

// The CE/CWE arm keeps its own guard: folding it into the match pattern would
// send table-coded fields down the generic arm instead.
#[allow(clippy::collapsible_match)]
fn check_datatype(
    fs: &spec::FieldSpec,
    rep: Repetition<'_>,
    loc: &str,
    seg: &Segment<'_>,
    index: usize,
    out: &mut Vec<Finding>,
) {
    let c1 = rep.comp_text(1);
    let mut bad = |summary: String, detail: String| {
        out.push(
            Finding::new(
                Severity::Error,
                Category::DataType,
                loc.to_string(),
                summary,
                detail,
            )
            .at(seg, index),
        );
    };

    match fs.dt {
        "DTM" | "TS" => {
            if let Err(e) = datetime::parse_ts(c1) {
                bad("not a valid date/time".to_string(), e);
            }
        }
        "DT" => {
            if let Err(e) = datetime::parse_date(c1) {
                bad("not a valid date".to_string(), e);
            }
        }
        "TM" => {
            if let Err(e) = datetime::parse_time(c1) {
                bad("not a valid time".to_string(), e);
            }
        }
        "NM" => {
            if c1.parse::<f64>().is_err() {
                bad(
                    "not numeric".to_string(),
                    format!("{c1:?} cannot be read as a number"),
                );
            }
        }
        "SI" => match c1.parse::<u32>() {
            Ok(_) => {}
            Err(_) => bad(
                "not a valid sequence ID".to_string(),
                format!("{c1:?} is not a non-negative integer"),
            ),
        },
        "CX" => {
            if c1.is_empty() {
                bad(
                    "no ID value".to_string(),
                    "component 1 (ID Number) is empty while the field is populated".to_string(),
                );
            }
        }
        "EI" => {
            if c1.is_empty() {
                bad(
                    "no entity identifier".to_string(),
                    "component 1 is empty while the field is populated".to_string(),
                );
            }
        }
        "XPN" | "XCN" => {
            let family = if fs.dt == "XCN" { rep.comp_text(2) } else { c1 };
            let given = if fs.dt == "XCN" {
                rep.comp_text(3)
            } else {
                rep.comp_text(2)
            };
            if family.is_empty() && !given.is_empty() {
                out.push(
                    Finding::new(
                        Severity::Warning,
                        Category::DataType,
                        loc.to_string(),
                        "given name without a family name",
                        "the family name component is empty",
                    )
                    .at(seg, index),
                );
            }
            if fs.dt == "XCN" && c1.is_empty() && family.is_empty() {
                out.push(
                    Finding::new(
                        Severity::Warning,
                        Category::DataType,
                        loc.to_string(),
                        "neither an ID nor a name",
                        "components 1 (ID) and 2 (family name) are both empty",
                    )
                    .at(seg, index),
                );
            }
        }
        "PL" => {
            let point_of_care = c1;
            let filled = rep.filled_comps();
            if point_of_care.is_empty() && filled > 0 {
                bad(
                    "invalid location".to_string(),
                    "component 1 (point of care) is empty while later components are populated"
                        .to_string(),
                );
            } else if !rep.comp_text(3).is_empty() && rep.comp_text(2).is_empty() {
                out.push(
                    Finding::new(
                        Severity::Warning,
                        Category::DataType,
                        loc.to_string(),
                        "bed without a room",
                        "component 3 (bed) is populated but component 2 (room) is empty",
                    )
                    .at(seg, index),
                );
            }
        }
        "MSG" => {
            if rep.comp_text(2).is_empty() {
                out.push(
                    Finding::new(
                        Severity::Warning,
                        Category::DataType,
                        loc.to_string(),
                        "no trigger event",
                        "component 2 (trigger event) is empty",
                    )
                    .at(seg, index),
                );
            }
        }
        "VID" => {
            if !spec::is_known_version(c1) {
                bad(
                    "not a known HL7 version".to_string(),
                    format!("{c1:?} is not listed in HL7 table 0104"),
                );
            }
        }
        "CE" | "CWE" => {
            if !c1.is_empty() && rep.comp_text(3).is_empty() && fs.table.is_none() {
                out.push(
                    Finding::new(
                        Severity::Info,
                        Category::CodeTable,
                        loc.to_string(),
                        "no coding system",
                        format!("code {c1:?} is sent without component 3 (name of coding system)"),
                    )
                    .at(seg, index),
                );
            }
        }
        _ => {}
    }
}

fn check_table(
    table_id: &str,
    value: &str,
    loc: &str,
    field_name: &str,
    seg: &Segment<'_>,
    index: usize,
    out: &mut Vec<Finding>,
) {
    if value.is_empty() {
        return;
    }
    let Some(def) = spec::table(table_id) else {
        return;
    };
    if def.meaning(value).is_some() {
        return;
    }
    let severity = if def.closed {
        Severity::Error
    } else {
        Severity::Warning
    };
    let hint = if def.closed {
        format!(
            "permitted values: {}",
            def.codes
                .iter()
                .map(|(c, _)| *c)
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        format!("not listed in HL7 table {} ({})", def.id, def.name)
    };
    out.push(
        Finding::new(
            severity,
            Category::CodeTable,
            loc.to_string(),
            format!("unrecognised code {value:?}"),
            format!("{field_name}: {hint}"),
        )
        .at(seg, index),
    );
}

// ------------------------------------------------------------- cross-field checks

/// MSH self-consistency: control ID length, message time, declared character set.
struct MessageHeaderRule;

impl Rule for MessageHeaderRule {
    fn check(&self, msg: &Message<'_>, out: &mut Vec<Finding>) {
        let (ty, tm, td) = datetime::today();
        let msh = msg.msh();
        let control = msg.control_id();
        if control.chars().count() > 20 {
            out.push(
                Finding::new(
                    Severity::Warning,
                    Category::DataType,
                    "MSH-10",
                    "message control ID is longer than 20 characters",
                    format!(
                        "{} characters; receivers may truncate it",
                        control.chars().count()
                    ),
                )
                .at(msh, MSH_INDEX),
            );
        }

        if let Ok(ts) = datetime::parse_ts(msh.comp(7, 1)) {
            if ts.days_after(ty, tm, td) > 1 {
                out.push(
                    Finding::new(
                        Severity::Warning,
                        Category::Consistency,
                        "MSH-7",
                        "message date/time is in the future",
                        format!("{} is later than today", ts.display()),
                    )
                    .at(msh, MSH_INDEX),
                );
            }
            if ts.precision < Precision::Minute {
                out.push(
                    Finding::new(
                        Severity::Warning,
                        Category::DataType,
                        "MSH-7",
                        "message date/time has no time component",
                        "MSH-7 should carry at least YYYYMMDDHHMM",
                    )
                    .at(msh, MSH_INDEX),
                );
            }
        }

        // Non-ASCII payload without a declared character set is a classic interface bug.
        if msh.comp(18, 1).is_empty() && msg.segments.iter().any(|s| !s.raw.is_ascii()) {
            out.push(
                Finding::new(
                    Severity::Warning,
                    Category::Consistency,
                    "MSH-18",
                    "non-ASCII characters sent without a declared character set",
                    "populate MSH-18 (e.g. UNICODE UTF-8) so the receiver decodes correctly",
                )
                .at(msh, MSH_INDEX),
            );
        }
    }
}

/// EVN agreement with the trigger event and its own timestamps.
struct EventRule;

impl Rule for EventRule {
    fn check(&self, msg: &Message<'_>, out: &mut Vec<Finding>) {
        let (_, trigger, _) = msg.message_type();
        if let Some((i, evn)) = msg
            .segments
            .iter()
            .enumerate()
            .find(|(_, s)| s.name == "EVN")
        {
            let evn_type = evn.comp(1, 1);
            if !evn_type.is_empty() && !trigger.is_empty() && evn_type != trigger {
                out.push(
                    Finding::new(
                        Severity::Warning,
                        Category::Consistency,
                        "EVN-1",
                        format!("event type {evn_type:?} does not match MSH-9 trigger {trigger:?}"),
                        "the event type code should repeat the trigger event from MSH-9.2",
                    )
                    .at(evn, i),
                );
            }
            if let (Ok(recorded), Ok(occurred)) = (
                datetime::parse_ts(evn.comp(2, 1)),
                datetime::parse_ts(evn.comp(6, 1)),
            ) {
                if recorded.days_after(occurred.year, occurred.month, occurred.day) < 0 {
                    out.push(
                        Finding::new(
                            Severity::Warning,
                            Category::Consistency,
                            "EVN-2",
                            "event was recorded before it occurred",
                            format!(
                                "EVN-2 {} precedes EVN-6 {}",
                                recorded.display(),
                                occurred.display()
                            ),
                        )
                        .at(evn, i),
                    );
                }
            }
        }
    }
}

/// PID cross-field rules: birth, death and identifier list.
struct PatientRule;

impl Rule for PatientRule {
    fn check(&self, msg: &Message<'_>, out: &mut Vec<Finding>) {
        let (ty, tm, td) = datetime::today();
        if let Some((i, pid)) = msg
            .segments
            .iter()
            .enumerate()
            .find(|(_, s)| s.name == "PID")
        {
            if let Ok(dob) = datetime::parse_ts(pid.comp(7, 1)) {
                if dob.days_after(ty, tm, td) > 0 {
                    out.push(
                        Finding::new(
                            Severity::Error,
                            Category::Consistency,
                            "PID-7",
                            "date of birth is in the future",
                            format!("{} is later than today", dob.date_string()),
                        )
                        .at(pid, i),
                    );
                } else if dob.years_until(ty, tm, td) > 130 {
                    out.push(
                        Finding::new(
                            Severity::Warning,
                            Category::Consistency,
                            "PID-7",
                            "date of birth implies an implausible age",
                            format!("{} years old", dob.years_until(ty, tm, td)),
                        )
                        .at(pid, i),
                    );
                }
            }
            let death_date = pid.comp(29, 1);
            let death_flag = pid.comp(30, 1);
            if !death_date.is_empty() && death_flag != "Y" {
                out.push(
                    Finding::new(
                        Severity::Warning,
                        Category::Consistency,
                        "PID-30",
                        "death date is present but the death indicator is not Y",
                        format!("PID-29 is {death_date:?} while PID-30 is {death_flag:?}"),
                    )
                    .at(pid, i),
                );
            }
            if death_flag == "Y" && death_date.is_empty() {
                out.push(
                    Finding::new(
                        Severity::Info,
                        Category::Consistency,
                        "PID-29",
                        "patient is flagged as deceased without a death date",
                        "PID-30 is Y but PID-29 is empty",
                    )
                    .at(pid, i),
                );
            }
            // Duplicate identifiers in PID-3 confuse downstream matching.
            if let Some(field) = pid.field(3) {
                let ids: Vec<String> = field
                    .reps()
                    .map(|r| format!("{}|{}", r.comp_text(1), r.comp_text(4)))
                    .collect();
                for (n, id) in ids.iter().enumerate() {
                    if !id.starts_with('|') && ids[..n].contains(id) {
                        out.push(
                            Finding::new(
                                Severity::Warning,
                                Category::Consistency,
                                "PID-3",
                                "patient identifier list contains a duplicate",
                                format!("repetition {} repeats an earlier identifier", n + 1),
                            )
                            .at(pid, i),
                        );
                        break;
                    }
                }
            }
        }
    }
}

/// PV1 cross-field rules: class, location and visit dates.
struct VisitRule;

impl Rule for VisitRule {
    fn check(&self, msg: &Message<'_>, out: &mut Vec<Finding>) {
        let (code, trigger, _) = msg.message_type();
        if let Some((i, pv1)) = msg
            .segments
            .iter()
            .enumerate()
            .find(|(_, s)| s.name == "PV1")
        {
            let class = pv1.comp(2, 1);
            if matches!(class, "I" | "E" | "B") && !pv1.has(3) {
                out.push(
                    Finding::new(
                        Severity::Error,
                        Category::Consistency,
                        "PV1-3",
                        "assigned patient location is missing",
                        format!(
                            "patient class {:?} ({}) requires a location in PV1-3",
                            class,
                            spec::code_meaning("0004", class).unwrap_or("unknown class")
                        ),
                    )
                    .at(pv1, i),
                );
            }
            let admit = datetime::parse_ts(pv1.comp(44, 1));
            let discharge = datetime::parse_ts(pv1.comp(45, 1));
            if let (Ok(a), Ok(d)) = (&admit, &discharge) {
                if d.days_after(a.year, a.month, a.day) < 0 {
                    out.push(
                        Finding::new(
                            Severity::Error,
                            Category::Consistency,
                            "PV1-45",
                            "discharge date/time precedes admit date/time",
                            format!("PV1-44 {}, PV1-45 {}", a.display(), d.display()),
                        )
                        .at(pv1, i),
                    );
                }
            }
            if code == "ADT" && trigger == "A03" && discharge.is_err() {
                out.push(
                    Finding::new(
                        Severity::Error,
                        Category::Consistency,
                        "PV1-45",
                        "discharge date/time is missing",
                        "ADT^A03 (Discharge / End Visit) must carry PV1-45",
                    )
                    .at(pv1, i),
                );
            }
        }
    }
}

/// OBX values against the type declared alongside them.
struct ObservationRule;

impl Rule for ObservationRule {
    fn check(&self, msg: &Message<'_>, out: &mut Vec<Finding>) {
        for (i, obx) in msg
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.name == "OBX")
        {
            let vt = obx.comp(2, 1);
            let value = obx.text(5);
            if vt.is_empty() && !value.is_empty() {
                out.push(
                    Finding::new(
                        Severity::Warning,
                        Category::Consistency,
                        "OBX-2",
                        "observation value sent without a value type",
                        "OBX-2 must say how OBX-5 should be interpreted",
                    )
                    .at(obx, i),
                );
                continue;
            }
            if value.is_empty() {
                continue;
            }
            let first = obx.comp(5, 1);
            let mismatch = match vt {
                "NM" => first
                    .parse::<f64>()
                    .is_err()
                    .then(|| "not a number".to_string()),
                "DT" => datetime::parse_date(first).err(),
                "TS" | "DTM" => datetime::parse_ts(first).err(),
                "TM" => datetime::parse_time(first).err(),
                "SN" => {
                    // Structured numeric: <comparator><num1><sep><num2>
                    let num = obx.comp(5, 2);
                    (!num.is_empty() && num.parse::<f64>().is_err())
                        .then(|| "component 2 is not a number".to_string())
                }
                _ => None,
            };
            if let Some(reason) = mismatch {
                out.push(
                    Finding::new(
                        Severity::Error,
                        Category::Consistency,
                        "OBX-5",
                        format!("value does not match declared type {vt}"),
                        format!("{first:?}: {reason}"),
                    )
                    .at(obx, i),
                );
            }
        }
    }
}

/// Set IDs on repeating segments.
struct SetIdRule;

impl Rule for SetIdRule {
    fn check(&self, msg: &Message<'_>, out: &mut Vec<Finding>) {
        check_set_ids(msg, out);
    }
}

/// Repeating segments carry a 1-based Set ID in field 1; gaps and repeats there
/// break receivers that key on it.
fn check_set_ids(msg: &Message<'_>, out: &mut Vec<Finding>) {
    const SET_ID_SEGMENTS: &[&str] = &[
        "NK1", "AL1", "DG1", "PR1", "GT1", "IN1", "OBX", "FT1", "IAM", "RGS", "AIS", "AIL", "AIP",
        "NTE",
    ];
    for name in SET_ID_SEGMENTS {
        let occurrences: Vec<(usize, &Segment<'_>)> = msg
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.name == *name)
            .collect();
        if occurrences.len() < 2 {
            continue;
        }
        for (n, (i, seg)) in occurrences.iter().enumerate() {
            let raw = seg.comp(1, 1);
            if raw.is_empty() {
                continue;
            }
            let Ok(value) = raw.parse::<usize>() else {
                continue;
            };
            if value != n + 1 {
                out.push(
                    Finding::new(
                        Severity::Warning,
                        Category::Consistency,
                        format!("{name}-1"),
                        "set ID is out of sequence",
                        format!("occurrence {} carries set ID {}", n + 1, value),
                    )
                    .at(seg, *i),
                );
                break;
            }
        }
    }
}

/// Numeric field position inside a location like `PID-11` or `OBX-5[2]`, used
/// only for ordering findings.
fn field_seq(location: &str) -> usize {
    location.split_once('-').map_or(0, |(_, rest)| {
        rest.chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .unwrap_or(0)
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "panicking is the failure mode a test wants"
    )]
    use super::*;
    use crate::parser::parse_str;

    fn report(text: &str) -> Report {
        validate(&parse_str(text))
    }

    fn find<'a>(r: &'a Report, location: &str) -> Option<&'a Finding> {
        r.findings.iter().find(|f| f.location == location)
    }

    fn located(r: &Report, location: &str) -> Severity {
        find(r, location)
            .unwrap_or_else(|| panic!("expected a finding at {}: {:#?}", location, r.findings))
            .severity
    }

    const HEADER: &str =
        "MSH|^~\\&|HIS|MERCY|LIS|LAB|20240115143200||ADT^A01^ADT_A01|MSG1|P|2.5.1\r";

    /// Builds `NAME|f1|f2|...` from sparse (position, value) pairs so tests can
    /// name the field numbers they care about instead of counting pipes.
    fn segment(name: &str, fields: &[(usize, &str)]) -> String {
        let top = fields.iter().map(|(i, _)| *i).max().unwrap_or(0);
        let mut parts = vec![String::new(); top];
        for (i, value) in fields {
            parts[i - 1] = (*value).to_string();
        }
        format!("{}|{}\r", name, parts.join("|"))
    }

    /// A valid ADT^A01 whose PV1 can be extended with extra fields.
    fn adt(extra_pv1: &[(usize, &str)]) -> String {
        let mut pv1: Vec<(usize, &str)> = vec![
            (1, "1"),
            (2, "I"),
            (3, "ER^101^A"),
            (7, "1^Adams^Alice"),
            (19, "V1"),
            (44, "20240115143000"),
        ];
        pv1.retain(|(i, _)| !extra_pv1.iter().any(|(j, _)| j == i));
        pv1.extend_from_slice(extra_pv1);
        format!(
            "{HEADER}EVN|A01|20240115143200||||20240115143000\r\
PID|1||123456^^^MERCY^MR||Smith^John||19850312|M|||1 Oak St^^Springfield^IL^62704\r{}",
            segment("PV1", &pv1)
        )
    }

    #[test]
    fn every_rule_stands_on_its_own() {
        // Each rule must cope with any message, including one missing the
        // segments and fields it is interested in.
        let messages = [
            adt(&[]),
            HEADER.to_string(),
            format!("{HEADER}PID|1\rOBX|1|NM|X^Y^L||text\r"),
            "MSH|^~\\&|A|B|C|D|20240115143200||ACK|1|P|2.5.1\rMSA|AA|1\r".to_string(),
        ];
        for text in messages {
            let msg = parse_str(&text);
            for rule in RULES {
                let mut findings = Vec::new();
                rule.check(&msg, &mut findings);
            }
        }
    }

    #[test]
    fn validate_collects_from_every_rule() {
        let text = adt(&[]);
        let msg = parse_str(&text);
        let combined = validate(&msg).findings.len();
        let separate: usize = RULES
            .iter()
            .map(|rule| {
                let mut findings = Vec::new();
                rule.check(&msg, &mut findings);
                findings.len()
            })
            .sum();
        assert_eq!(
            combined, separate,
            "validate() must run each rule exactly once"
        );
    }

    #[test]
    fn a_complete_message_passes() {
        let r = report(&adt(&[]));
        let blocking: Vec<&Finding> = r
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .collect();
        assert!(blocking.is_empty(), "unexpected errors: {blocking:#?}");
    }

    #[test]
    fn missing_required_segment_is_an_error() {
        let r = report(&format!("{HEADER}PID|1||1^^^A^MR||Smith^John\r"));
        assert_eq!(located(&r, "EVN"), Severity::Error);
        assert_eq!(located(&r, "PV1"), Severity::Error);
    }

    #[test]
    fn missing_required_field_is_an_error_and_recommended_is_a_warning() {
        let r = report(&format!(
            "{HEADER}EVN|A01|20240115143200\rPID|1\rPV1|1|I|ER^101\r"
        ));
        assert_eq!(
            located(&r, "PID-3"),
            Severity::Error,
            "identifier list is required"
        );
        assert_eq!(
            located(&r, "PID-5"),
            Severity::Error,
            "patient name is required"
        );
        assert_eq!(
            located(&r, "PID-11"),
            Severity::Warning,
            "address is recommended"
        );
    }

    #[test]
    fn invalid_dates_are_reported_against_their_field() {
        let r = report(&format!(
            "{HEADER}EVN|A01|20240115143200\rPID|1||1^^^A^MR||Smith^John||19850332|M|||1 St^^X^IL^1\rPV1|1|I|ER^101\r"
        ));
        let f = find(&r, "PID-7").unwrap();
        assert_eq!(f.severity, Severity::Error);
        assert!(f.detail.contains("day 32"), "{}", f.detail);
    }

    #[test]
    fn future_birth_date_is_rejected() {
        let (y, _, _) = datetime::today();
        let text = format!(
            "{HEADER}EVN|A01|20240115143200\rPID|1||1^^^A^MR||Smith^John||{}0312|M|||1 St^^X^IL^1\rPV1|1|I|ER^101\r",
            y + 2
        );
        assert_eq!(located(&report(&text), "PID-7"), Severity::Error);
    }

    #[test]
    fn location_without_a_point_of_care_is_invalid() {
        let r = report(&format!(
            "{HEADER}EVN|A01|20240115143200\rPID|1||1^^^A^MR||Smith^John||19850312|M|||1 St^^X^IL^1\rPV1|1|I|^101^A\r"
        ));
        let f = find(&r, "PV1-3").unwrap();
        assert_eq!(f.severity, Severity::Error);
        assert_eq!(f.summary, "invalid location");
    }

    #[test]
    fn inpatient_without_a_location_is_an_error() {
        let r = report(&format!(
            "{HEADER}EVN|A01|20240115143200\rPID|1||1^^^A^MR||Smith^John||19850312|M|||1 St^^X^IL^1\rPV1|1|I\r"
        ));
        let f = find(&r, "PV1-3").unwrap();
        assert_eq!(f.severity, Severity::Error);
        assert!(f.detail.contains("Inpatient"), "{}", f.detail);
    }

    #[test]
    fn unknown_table_codes_warn_and_closed_tables_fail() {
        let r = report(&adt(&[]));
        assert!(find(&r, "PV1-2").is_none());

        let bad_sex = report(&format!(
            "{HEADER}EVN|A01|20240115143200\rPID|1||1^^^A^MR||Smith^John||19850312|Q|||1 St^^X^IL^1\rPV1|1|I|ER^101\r"
        ));
        assert_eq!(located(&bad_sex, "PID-8"), Severity::Warning);

        let bad_processing = "MSH|^~\\&|A|B|C|D|20240115143200||ACK|1|X|2.5.1\rMSA|AA|1\r";
        assert_eq!(located(&report(bad_processing), "MSH-11"), Severity::Error);
    }

    #[test]
    fn unknown_version_is_an_error() {
        let text = "MSH|^~\\&|A|B|C|D|20240115143200||ACK|1|P|9.9\rMSA|AA|1\r";
        assert_eq!(located(&report(text), "MSH-12"), Severity::Error);
    }

    #[test]
    fn observation_value_must_match_its_declared_type() {
        let numeric = "MSH|^~\\&|A|B|C|D|20240115143200||ORU^R01|1|P|2.5.1\r\
OBR|1|P1|F1|CBC^Complete Blood Count^L|||20240115150000\r\
OBX|1|NM|718-7^Hemoglobin^LN||thirteen|g/dL|||||F|||20240115150000\r";
        let numeric_report = report(numeric);
        let f = find(&numeric_report, "OBX-5").unwrap();
        assert_eq!(f.severity, Severity::Error);

        let ok = numeric.replace("thirteen", "13.4");
        assert!(find(&report(&ok), "OBX-5").is_none());
    }

    #[test]
    fn discharge_before_admit_is_an_error() {
        let r = report(&adt(&[(44, "20240115143000"), (45, "20240114120000")]));
        assert_eq!(located(&r, "PV1-45"), Severity::Error);
    }

    #[test]
    fn discharge_message_needs_a_discharge_time() {
        let text = "MSH|^~\\&|HIS|MERCY|LIS|LAB|20240115143200||ADT^A03^ADT_A03|MSG1|P|2.5.1\r\
EVN|A03|20240115143200||||20240115143000\r\
PID|1||1^^^A^MR||Smith^John||19850312|M|||1 St^^X^IL^1\r\
PV1|1|I|ER^101|||1^Adams^Alice||||||||||V1\r";
        assert_eq!(located(&report(text), "PV1-45"), Severity::Error);
    }

    #[test]
    fn out_of_order_segments_warn_but_legal_repeats_do_not() {
        let jumbled = format!(
            "{HEADER}PID|1||1^^^A^MR||Smith^John||19850312|M|||1 St^^X^IL^1\r\
EVN|A01|20240115143200||||20240115143000\rPV1|1|I|ER^101|||1^Adams^Alice||||||||||V1\r"
        );
        assert_eq!(located(&report(&jumbled), "EVN"), Severity::Warning);

        // NTE is allowed both before and after OBX in ORU^R01.
        let oru = "MSH|^~\\&|A|B|C|D|20240115143200||ORU^R01|1|P|2.5.1\r\
OBR|1|P1|F1|CBC^Complete Blood Count^L|||20240115150000\r\
OBX|1|NM|718-7^Hgb^LN||13.4|g/dL|||||F|||20240115150000\r\
NTE|1|L|Sample slightly haemolysed\r";
        assert!(find(&report(oru), "NTE").is_none());
    }

    #[test]
    fn unexpected_and_custom_segments() {
        let r = report(&format!("{}ZPD|1|local data\r", adt(&[])));
        assert!(find(&r, "ZPD").is_none(), "Z segments are site-defined");

        let r = report(&format!("{}MSA|AA|1\r", adt(&[])));
        assert_eq!(located(&r, "MSA"), Severity::Warning);
    }

    #[test]
    fn set_ids_must_count_up() {
        let text = format!(
            "{}AL1|1|DA|PEN^Penicillin^L\rAL1|3|DA|SUL^Sulfa^L\r",
            adt(&[])
        );
        assert_eq!(located(&report(&text), "AL1-1"), Severity::Warning);
    }

    #[test]
    fn repetition_in_a_single_value_field_warns() {
        let r = report(&format!(
            "{HEADER}EVN|A01|20240115143200\rPID|1||1^^^A^MR||Smith^John||19850312|M~F|||1 St^^X^IL^1\rPV1|1|I|ER^101\r"
        ));
        assert_eq!(located(&r, "PID-8"), Severity::Warning);
    }

    #[test]
    fn data_past_the_segment_definition_warns() {
        let r = report(&format!("{}NTE|1|L|note|type|extra\r", adt(&[])));
        assert!(r
            .findings
            .iter()
            .any(|f| f.summary.contains("beyond the last defined field")));
    }

    #[test]
    fn trigger_event_must_agree_with_evn() {
        let text = format!(
            "{HEADER}EVN|A04|20240115143200||||20240115143000\r\
PID|1||1^^^A^MR||Smith^John||19850312|M|||1 St^^X^IL^1\rPV1|1|I|ER^101|||1^A^B||||||||||V1\r"
        );
        assert_eq!(located(&report(&text), "EVN-1"), Severity::Warning);
    }

    #[test]
    fn explicit_null_satisfies_a_populated_field() {
        let r = report(&format!(
            "{HEADER}EVN|A01|20240115143200\rPID|1||1^^^A^MR||Smith^John||\"\"|M|||1 St^^X^IL^1\rPV1|1|I|ER^101\r"
        ));
        assert!(
            find(&r, "PID-7").is_none(),
            "\"\" is a deliberate null, not bad data"
        );
    }

    #[test]
    fn severity_rolls_up_per_segment_and_message() {
        let text = format!(
            "{HEADER}EVN|A01|20240115143200\rPID|1||1^^^A^MR||Smith^John||19850332|M|||1 St^^X^IL^1\rPV1|1|I|ER^101|||1^A^B||||||||||V1\r"
        );
        let msg = parse_str(&text);
        let r = validate(&msg);
        let pid = msg.segments.iter().position(|s| s.name == "PID").unwrap();
        assert_eq!(r.segment_severity(pid, false), Some(Severity::Error));
        assert_eq!(r.worst(), Some(Severity::Error));
        assert!(r.errors() >= 1);
    }
}
