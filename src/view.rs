//! The view model that both output modes share: one row per field (or field
//! repetition), already decoded and already carrying its validation severity.
//!
//! Building this model here keeps the printed report and the interactive viewer
//! from re-deriving the same rows in two different ways; each of them only
//! decides how a row is painted.

use std::fmt::Write as _;

use crate::datetime;
use crate::parser::{unescape, Message, Repetition, Separators};
use crate::spec::{self, Use};
use crate::validate::{Report, Severity};

/// One line of a segment's field table.
pub struct FieldRow {
    pub seq: usize,
    /// Field name from the dictionary, or `Field N` when it is not defined.
    pub label: String,
    /// The value as sent, with escape sequences resolved.
    pub value: String,
    /// The same value in plain language, when that adds anything.
    pub decoded: Option<String>,
    /// `Some(n)` for the nth repetition of a repeating field, `None` for the first.
    pub repetition: Option<usize>,
    /// Worst finding recorded against this field.
    pub severity: Option<Severity>,
    pub usage: Option<Use>,
    /// False when the sender left the field empty.
    pub present: bool,
}

impl FieldRow {
    /// Why an empty field is being shown at all.
    pub const fn empty_note(&self) -> &'static str {
        match self.usage {
            Some(Use::Required) => "required",
            Some(Use::Recommended) => "recommended",
            _ => "",
        }
    }
}

#[derive(Clone, Copy)]
pub struct RowOptions {
    /// Include fields the sender left empty and the standard does not insist on.
    pub show_empty: bool,
    /// Let informational findings colour a row.
    pub include_info: bool,
}

/// Builds the field table for one segment occurrence.
pub fn segment_rows(
    msg: &Message<'_>,
    seg_index: usize,
    report: &Report,
    options: RowOptions,
) -> Vec<FieldRow> {
    let seg = &msg.segments[seg_index];
    let sep = &msg.sep;
    let findings = report.for_segment(seg_index);

    let mut rows = Vec::new();
    for seq in field_positions(seg.name, seg.last_populated()) {
        let field_spec = spec::field_spec(seg.name, seq);
        let usage = field_spec.map(|f| f.usage);
        let expected = matches!(usage, Some(Use::Required | Use::Recommended));
        let present = seg.has(seq);
        if !present && !options.show_empty && !expected {
            continue;
        }

        let location = format!("{}-{}", seg.name, seq);
        let repetition_prefix = format!("{location}[");
        let severity = findings
            .iter()
            .filter(|f| f.location == location || f.location.starts_with(&repetition_prefix))
            .filter(|f| options.include_info || f.severity != Severity::Info)
            .map(|f| f.severity)
            .max();
        let label = spec::field_label(seg.name, seq);

        let Some(field) = seg.field(seq).filter(|_| present) else {
            rows.push(FieldRow {
                seq,
                label,
                value: String::new(),
                decoded: None,
                repetition: None,
                severity,
                usage,
                present: false,
            });
            continue;
        };

        for (index, rep) in field.reps().enumerate() {
            let value = unescape(rep.text(), sep).replace('\n', " \u{21b5} ");
            let decoded = humanize(field_spec, rep, sep).filter(|d| *d != value);
            rows.push(FieldRow {
                seq,
                label: label.clone(),
                value,
                decoded,
                repetition: (index > 0).then_some(index + 1),
                severity: if index == 0 { severity } else { None },
                usage,
                present: true,
            });
        }
    }
    rows
}

/// Field numbers worth showing: everything the sender populated, plus every
/// position the dictionary defines.
fn field_positions(segment: &str, last_populated: usize) -> Vec<usize> {
    let mut positions: Vec<usize> = (1..=last_populated).collect();
    if let Some(spec) = spec::segment_spec(segment) {
        positions.extend(spec.fields.iter().map(|f| f.seq));
    }
    positions.sort_unstable();
    positions.dedup();
    positions
}

// ------------------------------------------------------- data-type humanisers

/// Renders a composite value the way a human would read it, e.g. `Smith^John`
/// becomes `John Smith` and `19850312` becomes `1985-03-12, age 40`.
pub fn humanize(
    fs: Option<&spec::FieldSpec>,
    rep: Repetition<'_>,
    sep: &Separators,
) -> Option<String> {
    let fs = fs?;
    let c = |n: usize| unescape(rep.comp_text(n), sep);
    let c1 = c(1);
    if c1.is_empty() && rep.filled_comps() == 0 {
        return None;
    }
    let decoded = match fs.dt {
        "DTM" | "TS" => {
            let ts = datetime::parse_ts(&c1).ok()?;
            let mut s = ts.display();
            if fs.name.contains("Birth") {
                let (y, m, d) = datetime::today();
                let age = ts.years_until(y, m, d);
                if (0..=130).contains(&age) {
                    let _ = write!(s, ", age {age}");
                }
            }
            s
        }
        "DT" => datetime::parse_date(&c1).ok()?.display(),
        "PL" => {
            let mut parts = vec![c1.clone()];
            if !c(2).is_empty() {
                parts.push(format!("room {}", c(2)));
            }
            if !c(3).is_empty() {
                parts.push(format!("bed {}", c(3)));
            }
            if !c(4).is_empty() {
                parts.push(c(4));
            }
            parts.retain(|s| !s.is_empty());
            parts.join(", ")
        }
        "XPN" => person_name(&c(1), &c(2), &c(3), &c(4), &c(5)),
        "XCN" => {
            let name = person_name(&c(2), &c(3), &c(4), &c(5), &c(6));
            match (name.is_empty(), c1.is_empty()) {
                (true, true) => return None,
                (true, false) => format!("ID {c1}"),
                (false, true) => name,
                (false, false) => format!("{name} (ID {c1})"),
            }
        }
        "CX" => {
            let mut s = c1.clone();
            let kind = c(5);
            let authority = c(4);
            let mut extra: Vec<String> = Vec::new();
            if !kind.is_empty() {
                extra.push(kind);
            }
            if !authority.is_empty() {
                extra.push(authority);
            }
            if !extra.is_empty() {
                let _ = write!(s, " ({})", extra.join(", "));
            }
            s
        }
        "XAD" => {
            let parts: Vec<String> = [c(1), c(3), c(4), c(5), c(6)]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            parts.join(", ")
        }
        "XTN" => {
            let candidates = [c(1), c(4), c(12), c(7)];
            candidates.into_iter().find(|s| !s.is_empty())?
        }
        "HD" => {
            let parts: Vec<String> = [c(1), c(2)].into_iter().filter(|s| !s.is_empty()).collect();
            parts.join(" / ")
        }
        "CE" | "CWE" => {
            let text = c(2);
            let system = c(3);
            let mut s = if text.is_empty() {
                c1.clone()
            } else {
                format!("{text} ({c1})")
            };
            if !system.is_empty() {
                let _ = write!(s, " [{system}]");
            }
            s
        }
        "MSG" => return None,
        _ => {
            // Plain coded values decode through their table.
            let table = fs.table?;
            spec::code_meaning(table, &c1)?.to_string()
        }
    };
    // A table-coded composite still deserves its decoded meaning.
    let decoded = match fs.table {
        Some(t)
            if !matches!(
                fs.dt,
                "DTM" | "TS" | "DT" | "XPN" | "XCN" | "CX" | "XAD" | "XTN"
            ) =>
        {
            match spec::code_meaning(t, &c1) {
                // The bare code adds nothing once it is spelled out.
                Some(meaning) if decoded == c1 || decoded.is_empty() => meaning.to_string(),
                Some(meaning) if !decoded.contains(meaning) => {
                    format!("{meaning} \u{b7} {decoded}")
                }
                _ => decoded,
            }
        }
        _ => decoded,
    };
    if decoded.trim().is_empty() {
        None
    } else {
        Some(decoded)
    }
}

fn person_name(family: &str, given: &str, middle: &str, suffix: &str, prefix: &str) -> String {
    let parts: Vec<&str> = [prefix, given, middle, family, suffix]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    parts.join(" ")
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
PID|1||123456^^^MERCY^MR~987654321^^^SSA^SS||Smith^John^A^^Mr||19850312|M|||42 Oak St^^Springfield^IL^62704\r\
PV1|1|I|ER^101^A^MERCY|E|||1234^Adams^Alice^^^Dr||||||||||||V1|||||||||||||||||||||||||20240115143000\r";

    fn rows(segment: &str, show_empty: bool) -> Vec<FieldRow> {
        let msg = parse_str(MSG);
        let report = validate(&msg);
        let index = msg.segments.iter().position(|s| s.name == segment).unwrap();
        segment_rows(
            &msg,
            index,
            &report,
            RowOptions {
                show_empty,
                include_info: false,
            },
        )
    }

    fn humanized(segment: &str, seq: usize, msg: &Message<'_>) -> Option<String> {
        let seg = msg.first(segment).unwrap();
        humanize(
            spec::field_spec(segment, seq),
            seg.field(seq).unwrap().rep(1),
            &msg.sep,
        )
    }

    #[test]
    fn decodes_composite_values_for_reading() {
        let msg = parse_str(MSG);
        assert_eq!(humanized("PID", 5, &msg).unwrap(), "Mr John A Smith");
        assert_eq!(humanized("PID", 3, &msg).unwrap(), "123456 (MR, MERCY)");
        assert_eq!(humanized("PID", 8, &msg).unwrap(), "Male");
        assert_eq!(humanized("PV1", 2, &msg).unwrap(), "Inpatient");
        assert_eq!(
            humanized("PV1", 3, &msg).unwrap(),
            "ER, room 101, bed A, MERCY"
        );
        assert_eq!(
            humanized("PV1", 7, &msg).unwrap(),
            "Dr Alice Adams (ID 1234)"
        );
        assert_eq!(
            humanized("PID", 11, &msg).unwrap(),
            "42 Oak St, Springfield, IL, 62704"
        );
        assert!(humanized("PID", 7, &msg)
            .unwrap()
            .starts_with("1985-03-12, age "));
    }

    #[test]
    fn coded_values_are_not_repeated_after_their_meaning() {
        let text = format!("{MSG}AL1|1|DA|PEN^Penicillin^L|SV\r");
        let msg = parse_str(&text);
        assert_eq!(humanized("AL1", 2, &msg).unwrap(), "Drug allergy");
        assert_eq!(humanized("AL1", 3, &msg).unwrap(), "Penicillin (PEN) [L]");
    }

    #[test]
    fn one_row_per_repetition_with_the_first_carrying_the_severity() {
        let rows = rows("PID", false);
        let identifiers: Vec<&FieldRow> = rows.iter().filter(|r| r.seq == 3).collect();
        assert_eq!(identifiers.len(), 2);
        assert_eq!(identifiers[0].repetition, None);
        assert_eq!(identifiers[1].repetition, Some(2));
        assert_eq!(identifiers[1].severity, None);
        assert_eq!(identifiers[0].value, "123456^^^MERCY^MR");
    }

    #[test]
    fn empty_fields_appear_only_when_they_matter() {
        // A PID with nothing but a set ID: every other field is absent.
        let msg = parse_str("MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|1|P|2.5.1\rPID|1\r");
        let report = validate(&msg);
        let rows = |show_empty| {
            segment_rows(
                &msg,
                1,
                &report,
                RowOptions {
                    show_empty,
                    include_info: false,
                },
            )
        };

        let visible = rows(false);
        assert!(
            visible.iter().any(|r| r.seq == 11 && !r.present),
            "an empty but recommended field stays visible"
        );
        assert!(
            !visible.iter().any(|r| r.seq == 25),
            "an empty optional field stays hidden"
        );

        let everything = rows(true);
        assert!(everything.iter().any(|r| r.seq == 25 && !r.present));
        assert_eq!(
            everything
                .iter()
                .find(|r| r.seq == 11)
                .unwrap()
                .empty_note(),
            "recommended"
        );
        assert_eq!(
            everything.iter().find(|r| r.seq == 3).unwrap().empty_note(),
            "required"
        );
    }

    #[test]
    fn rows_carry_labels_and_decoded_values() {
        let rows = rows("PID", false);
        let name = rows.iter().find(|r| r.seq == 5).unwrap();
        assert_eq!(name.label, "Patient Name");
        assert_eq!(name.decoded.as_deref(), Some("Mr John A Smith"));
        let sex = rows.iter().find(|r| r.seq == 8).unwrap();
        assert_eq!(sex.decoded.as_deref(), Some("Male"));
    }

    #[test]
    fn undefined_fields_still_get_a_row() {
        let msg = parse_str(
            "MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|1|P|2.5.1\rNTE|1|L|note|type|extra\r",
        );
        let report = validate(&msg);
        let rows = segment_rows(
            &msg,
            1,
            &report,
            RowOptions {
                show_empty: false,
                include_info: false,
            },
        );
        let extra = rows.iter().find(|r| r.seq == 5).unwrap();
        assert_eq!(extra.label, "Field 5");
        assert_eq!(extra.value, "extra");
    }
}
