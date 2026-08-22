//! End-to-end tests: run the built binary the way a user would.
#![allow(
    clippy::unwrap_used,
    reason = "panicking is the failure mode a test wants"
)]

use std::io::Write;
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_hl7probe");

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("binary should run")
}

fn pipe(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(BIN)
        .args(args)
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary should run");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn code(out: &Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

#[test]
fn reports_a_clean_message_and_exits_zero() {
    let out = run(&["examples/adt_a01.hl7"]);
    let text = stdout(&out);
    assert_eq!(code(&out), 0, "{text}");
    assert!(text.contains("HL7 v2.5.1"));
    assert!(text.contains("ADT^A01"));
    assert!(text.contains("Admit / Visit Notification"));
    assert!(text.contains("Patient Name"));
    assert!(text.contains("John A Smith"));
    assert!(text.contains("message passes all checks"));
}

#[test]
fn invalid_message_lists_findings_and_exits_one() {
    let out = run(&["examples/invalid.hl7"]);
    let text = stdout(&out);
    assert_eq!(code(&out), 1, "{text}");
    for expected in [
        "required segment is missing",
        "PID-7",
        "not a valid date/time",
        "PV1-3",
        "invalid location",
        "OBX-5",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
    }
}

#[test]
fn strict_mode_fails_on_warnings_only() {
    let warning_only = "MSH|^~\\&|HIS|MERCY|LIS|LAB|20240115143200||ADT^A01|MSG1|P|2.5.1\r\
EVN|A01|20240115143200||||20240115143000\r\
PID|1||123456^^^MERCY^MR||Smith^John||19850312|M\r\
PV1|1|O|CLINIC^1\r";
    assert_eq!(code(&pipe(&["-q"], warning_only)), 0);
    assert_eq!(code(&pipe(&["-q", "--strict"], warning_only)), 1);
}

#[test]
fn unreadable_input_exits_two() {
    let out = run(&["no/such/file.hl7"]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("no/such/file.hl7"));

    let out = pipe(&[], "this is not an HL7 message\n");
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("no MSH segment"));
}

#[test]
fn quiet_mode_prints_one_line_per_message() {
    let out = run(&["-q", "examples/adt_a01.hl7", "examples/batch.hl7"]);
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 3, "{text}");
    assert!(lines[0].starts_with("adt_a01.hl7"));
    assert!(lines[1].starts_with("batch.hl7#1"));
    assert!(lines[2].starts_with("batch.hl7#2"));
}

#[test]
fn segment_filter_limits_the_field_tables() {
    let text = stdout(&run(&["-s", "PID", "examples/adt_a01.hl7"]));
    assert!(text.contains("Patient Identifier List"));
    assert!(
        !text.contains("Assigned Patient Location"),
        "PV1 table should be hidden"
    );
    assert!(
        text.contains("PV1"),
        "the segment overview still lists every segment"
    );
}

#[test]
fn field_query_prints_raw_values() {
    assert_eq!(
        stdout(&run(&["-f", "PID-5.1", "examples/adt_a01.hl7"])).trim(),
        "Smith"
    );
    assert_eq!(
        stdout(&run(&["-f", "MSH-10", "examples/adt_a01.hl7"])).trim(),
        "MSG00001"
    );
    assert_eq!(
        stdout(&run(&["-f", "OBX[2]-5", "examples/oru_r01.hl7"])).trim(),
        "39.1"
    );
    assert_eq!(
        stdout(&run(&["-f", "PV1-3.2", "examples/adt_a01.hl7"])).trim(),
        "101"
    );

    // Every repetition is printed when no repetition is named.
    let ids = stdout(&run(&["-f", "PID-3.1", "examples/adt_a01.hl7"]));
    assert_eq!(ids.lines().collect::<Vec<_>>(), vec!["123456", "987654321"]);

    // A field that is not there is a non-zero exit, not an error message.
    assert_eq!(code(&run(&["-f", "ZZZ-1", "examples/adt_a01.hl7"])), 1);
    assert_eq!(code(&run(&["-f", "nonsense", "examples/adt_a01.hl7"])), 2);
}

#[test]
fn json_output_is_machine_readable() {
    let out = run(&["--json", "examples/invalid.hl7"]);
    assert_eq!(code(&out), 1);
    let text = stdout(&out);
    let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    assert_eq!(value["status"], "error");
    let message = &value["files"][0]["messages"][0];
    assert_eq!(message["type"], "ADT^A01");
    assert_eq!(message["version"], "2.5.1");
    assert_eq!(message["control_id"], "MSG00002");
    assert!(message["summary"]["errors"].as_u64().unwrap() >= 4);
    let findings = message["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["location"] == "PV1-3" && f["severity"] == "error"));
    let segments = message["segments"].as_array().unwrap();
    assert_eq!(segments[0]["name"], "MSH");
    assert!(segments[0]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["name"] == "Version ID"));
}

#[test]
fn reads_a_message_from_a_pipe() {
    let out = pipe(
        &[],
        "MSH|^~\\&|A|B|C|D|20240115143200||ACK|1|P|2.5.1\rMSA|AA|MSG00001\r",
    );
    assert_eq!(code(&out), 0, "{}", stdout(&out));
    assert!(stdout(&out).contains("General Acknowledgment"));
}

#[test]
fn selects_a_single_message_from_a_batch() {
    let text = stdout(&run(&["-m", "2", "--summary", "examples/batch.hl7"]));
    assert!(text.contains("ADT^A03"));
    assert!(!text.contains("ADT^A01"));
    assert_eq!(code(&run(&["-m", "9", "examples/batch.hl7"])), 2);
}

#[test]
fn escape_sequences_are_resolved_for_display() {
    let out = pipe(
        &[],
        "MSH|^~\\&|A|B|C|D|20240115143200||ADT^A08|1|P|2.5.1\r\
EVN|A08|20240115143200||||20240115143000\r\
PID|1||1^^^A^MR||O\\T\\Brien^Se\\S\\an||19850312|M|||1 St^^X^IL^1\r\
PV1|1|O|CL^1\r",
    );
    let text = stdout(&out);
    assert!(text.contains("O&Brien^Se^an"), "{}", text);
}

#[test]
fn help_and_version_are_available() {
    let help = stdout(&run(&["--help"]));
    assert!(help.contains("Reads HL7 v2.x messages"));
    assert!(help.contains("Exit status: 0 clean, 1 validation errors, 2 unreadable input"));
    assert!(help.contains("--tui"));
    assert!(help.contains("EXAMPLES"));
    assert!(stdout(&run(&["--version"])).contains(env!("CARGO_PKG_VERSION")));
}

/// A batch whose second message has an unusable MSH, so the parse error and
/// the two readable messages have to coexist.
fn batch_with_a_broken_message() -> String {
    let good = std::fs::read_to_string("examples/adt_a01.hl7")
        .unwrap()
        .replace('\n', "\r");
    format!("{good}MSHzzzz\rPID|1||X\r{good}")
}

#[test]
fn parse_errors_print_before_the_messages_they_sit_among() {
    let out = pipe(&[], &batch_with_a_broken_message());
    let text = stdout(&out);
    assert_eq!(code(&out), 2, "{text}");

    let error_at = text
        .find("parse error")
        .expect("the bad message is reported");
    let first_message_at = text
        .find("message 1 of")
        .expect("the good ones still render");
    assert!(
        error_at < first_message_at,
        "the error belongs above the messages:\n{text}"
    );
}

#[test]
fn message_numbering_counts_only_the_readable_messages() {
    let text = stdout(&pipe(&[], &batch_with_a_broken_message()));
    assert!(text.contains("message 1 of 2"), "{text}");
    assert!(text.contains("message 2 of 2"), "{text}");
    // The unreadable one must not claim a number, or push the last one past
    // the total.
    assert!(!text.contains("message 3 of 2"), "{text}");
    assert!(!text.contains("of 3"), "{text}");

    let quiet = stdout(&pipe(&["-q"], &batch_with_a_broken_message()));
    assert!(quiet.contains("#1"), "{quiet}");
    assert!(quiet.contains("#2"), "{quiet}");
    assert!(!quiet.contains("#3"), "{quiet}");
}

#[test]
fn a_closed_pipe_is_not_an_error() {
    // `hl7probe big.hl7 | head` closes stdout early; that ends the output
    // rather than failing the run.
    let batch = batch_with_a_broken_message().repeat(200);
    let child = Command::new(BIN)
        .args(["-"])
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut child = child;
    let _ = child.stdin.as_mut().unwrap().write_all(batch.as_bytes());
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(!err.contains("stdout:"), "{err}");
}

#[test]
fn json_reports_a_batch_with_an_unreadable_message() {
    let out = pipe(&["--json"], &batch_with_a_broken_message());
    let text = stdout(&out);
    assert_eq!(code(&out), 2, "{text}");
    let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");

    // The document is written as the messages are parsed, so the trailing
    // status has to reflect everything that came before it.
    assert_eq!(value["status"], "error");
    let file = &value["files"][0];
    assert_eq!(file["parse_errors"].as_array().unwrap().len(), 1);
    let messages = file["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2, "only the readable ones are described");
    for message in messages {
        assert_eq!(message["type"], "ADT^A01");
    }
}

#[test]
fn json_status_carries_the_worst_severity_in_the_batch() {
    let clean = std::fs::read_to_string("examples/adt_a01.hl7")
        .unwrap()
        .replace('\n', "\r");
    let broken = std::fs::read_to_string("examples/invalid.hl7")
        .unwrap()
        .replace('\n', "\r");

    let value: serde_json::Value =
        serde_json::from_str(&stdout(&pipe(&["--json"], &clean))).unwrap();
    assert_eq!(value["status"], "ok");

    // The failing message is last, so status can only be right if the walk
    // finished before status was written.
    let out = pipe(&["--json"], &format!("{clean}{broken}"));
    let value: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(value["status"], "error");
    assert_eq!(code(&out), 1);
}

#[test]
fn a_field_query_survives_a_closed_pipe() {
    // This used to panic: the query printed with `println!`, which fails hard
    // when the reader has gone away.
    let batch = std::fs::read_to_string("examples/adt_a01.hl7")
        .unwrap()
        .replace('\n', "\r")
        .repeat(400);
    let mut child = Command::new(BIN)
        .args(["-f", "PID-5.1", "-"])
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = child.stdin.as_mut().unwrap().write_all(batch.as_bytes());
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(!err.contains("panicked"), "{err}");
}
