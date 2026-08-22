# hl7probe

**A friendly command-line tool for reading and checking HL7 v2 messages.**

[![CI](https://github.com/sudhi001/hl7probe/actions/workflows/ci.yml/badge.svg)](https://github.com/sudhi001/hl7probe/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/sudhi001/hl7probe?sort=semver)](https://github.com/sudhi001/hl7probe/releases)
[![crates.io](https://img.shields.io/crates/v/hl7probe?logo=rust)](https://crates.io/crates/hl7probe)
[![Downloads](https://img.shields.io/github/downloads/sudhi001/hl7probe/total?label=downloads)](https://github.com/sudhi001/hl7probe/releases)
[![Stars](https://img.shields.io/github/stars/sudhi001/hl7probe?label=stars)](https://github.com/sudhi001/hl7probe/stargazers)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Hospital systems talk to each other in HL7 v2 — dense lines of text full of
pipes and carets that look like this:

```
PID|1||123456^^^MERCY^MR||Smith^John^A^^Mr||19850312|M
```

Reading that by hand is slow and error-prone. `hl7probe` turns those lines into
something a person can read, and tells you what a receiving hospital system
would reject.

<p align="center">
  <img src="https://raw.githubusercontent.com/sudhi001/hl7probe/main/docs/demo.gif" alt="hl7probe decoding a message, pulling out a field and catching errors in a broken one" width="760">
</p>

---

## Contents

- [Why you'd use it](#why-youd-use-it)
- [Install](#install)
- [Quick start](#quick-start)
- [Reading the output](#reading-the-output)
- [Interactive viewer](#interactive-viewer)
- [Using it in scripts and CI](#using-it-in-scripts-and-ci)
- [All the options](#all-the-options)
- [What it checks](#what-it-checks)
- [What it accepts](#what-it-accepts)
- [Performance](#performance)
- [Building from source](#building-from-source)
- [Contributing](#contributing)
- [License](#license)

## Why you'd use it

You are wiring up an interface between two health systems and a message is
being rejected. You need to know **what is in the message** and **what is wrong
with it** — quickly, without opening a heavyweight integration engine.

`hl7probe` answers both in one command:

- Every field is shown with its real name — `PID-5` becomes *Patient Name*.
- Codes are translated — `M` becomes *Male*, `I` becomes *Inpatient*.
- Dates become readable — `19850312` becomes *1985-03-12, age 41*.
- Problems are listed with the exact field, the line number and why it matters.

No configuration, no database, no server. One binary, one file, one answer.

## Install

### Homebrew (macOS and Linux)

```sh
brew install sudhi001/tap/hl7probe
```

Recent Homebrew versions ask you to trust a third-party tap the first time; if
you see that prompt, run `brew trust sudhi001/tap` and install again.

### Download a prebuilt binary

Grab the archive for your platform from the
[releases page](https://github.com/sudhi001/hl7probe/releases), unpack it and
put `hl7probe` somewhere on your `PATH`. The Linux builds are static, so they
run on any distribution regardless of its glibc version:

```sh
shasum -a 256 -c hl7probe-*.tar.gz.sha256      # optional: verify the download
tar xzf hl7probe-*.tar.gz
sudo mv hl7probe-*/hl7probe /usr/local/bin/
```

On Windows, download the `x86_64-pc-windows-msvc.zip` archive, unpack it and put
`hl7probe.exe` in a folder on your `PATH`:

```powershell
Expand-Archive hl7probe-*-x86_64-pc-windows-msvc.zip -DestinationPath .
```

### With Cargo

```sh
cargo install hl7probe
```

Check it works:

```sh
hl7probe --version
```

## Quick start

Point it at a message file:

```sh
hl7probe message.hl7
```

Or pipe one in:

```sh
cat message.hl7 | hl7probe
```

Try it on the samples that ship with the project:

```sh
hl7probe examples/adt_a01.hl7     # a healthy admission message
hl7probe examples/invalid.hl7     # one with deliberate mistakes
hl7probe examples/oru_r01.hl7     # a lab result
hl7probe examples/batch.hl7       # a file holding several messages
```

## Reading the output

<p align="center">
  <img src="https://raw.githubusercontent.com/sudhi001/hl7probe/main/docs/report.svg" alt="a decoded message with its validation findings" width="700">
</p>

The report has three parts.

**1. What the message is.** The HL7 version, the message type, a plain-English
description, who sent it and when.

```
HL7 v2.5.1   ADT^A01   Admit / Visit Notification
MSG00001  ·  2024-01-15 14:32:00  ·  HIS/MERCY → LIS/LAB  ·  Production
```

**2. What is inside it.** Each segment is listed with a status mark, then each
field is shown with its name, its raw value, and — after the `›` — the same
value in plain language.

```
Segments
────────────────────────────────────────────
MSH ✓  Message Header
EVN ✓  Event Type
PID ✓  Patient Identification
PV1 ✓  Patient Visit

PID · Patient Identification   line 3
──────────────────────────────────────────────────────────────
    3  Patient Identifier List  123456^^^MERCY^MR   › 123456 (MR, MERCY)
       ~ rep 2                  987654321^^^SSA^SS  › 987654321 (SS, SSA)
    5  Patient Name             Smith^John^A^^Mr    › Mr John A Smith
    7  Date/Time of Birth       19850312            › 1985-03-12, age 41
    8  Administrative Sex       M                   › Male
⚠  11  Patient Address                              (empty)  recommended
```

**3. What is wrong with it.** Five groups of checks, then the individual
findings, each pointing at the field responsible.

```
Validation
────────────────────────────────────────────
✗ Structure         ADT^A01
⚠ Required fields
✗ Data types
⚠ Code tables
✗ Consistency

✗ EVN     required segment is missing  — ADT^A01 requires EVN (Event Type)
✗ PID-7   not a valid date/time  — day 32 does not exist in 1985-03
✗ PV1-3   invalid location  — component 1 (point of care) is empty
⚠ PID-11  missing  — Patient Address should be populated when the value is known

5 errors  ·  8 warnings
```

The three marks mean:

| Mark | Meaning |
| :---: | --- |
| ✓ | Fine |
| ⚠ | Works, but a receiving system may complain — a missing recommended field, an unusual code |
| ✗ | Wrong — this will be rejected |

## Interactive viewer

For bigger messages, browse instead of scroll:

```sh
hl7probe --tui message.hl7
```

<p align="center">
  <img src="https://raw.githubusercontent.com/sudhi001/hl7probe/main/docs/tui.svg" alt="the hl7probe interactive viewer" width="820">
</p>

Segments on the left, decoded fields on the right, problems underneath. Move
with the arrow keys, press `?` for help and `q` to quit.

| Key | Action |
| --- | --- |
| `↑` `↓` or `j` `k` | Move within the focused panel |
| `←` `→` or `h` `l` | Jump between segments and fields |
| `tab` | Cycle segments → fields → validation |
| `n` / `p` | Next / previous message in the file |
| `a` | Also show fields that were left empty |
| `v` | Include informational notes |
| `r` | Show the raw segment line |
| `f` | Findings for this segment only, or the whole message |
| `?` | Help |
| `q` or `esc` | Quit |

## Using it in scripts and CI

**One line per message**, ideal for checking a folder full of test messages:

```sh
$ hl7probe -q outbound/*.hl7
adt_a01.hl7  ADT^A01  ✓ message passes all checks
batch.hl7#1  ADT^A01  ✓ message passes all checks
batch.hl7#2  ADT^A03  ✓ message passes all checks
invalid.hl7  ADT^A01  5 errors  ·  8 warnings
```

**Exit codes** make it a gate in a build pipeline:

| Code | Meaning |
| :---: | --- |
| `0` | No errors (warnings are allowed unless you pass `--strict`) |
| `1` | At least one validation error |
| `2` | The input could not be read, or contained no HL7 message |

```sh
hl7probe --strict outbound/*.hl7 || exit 1
```

**Pull out a single value** without writing a parser:

```sh
$ hl7probe -f PID-5.1 message.hl7        # family name
Smith
$ hl7probe -f PID-3 message.hl7          # every patient identifier
123456^^^MERCY^MR
987654321^^^SSA^SS
$ hl7probe -f 'OBX[2]-5' results.hl7     # value of the second OBX segment
39.1
```

HL7 escape sequences are already decoded, so the output drops straight into a
shell script.

**Machine-readable reports** for dashboards and tests:

```sh
hl7probe --json message.hl7 | jq '.files[].messages[].findings[] | select(.severity == "error")'
```

## All the options

```
hl7probe [OPTIONS] [FILE]...
```

`FILE` can be given more than once. Use `-`, or no file at all, to read from
standard input.

| Option | What it does |
| --- | --- |
| `-t`, `--tui` | Open the interactive viewer |
| `--json` | Print the whole report as JSON |
| `-q`, `--quiet` | Print one verdict line per message |
| `-v`, `--verbose` | Include informational notes |
| `-a`, `--all` | Show fields that were left empty |
| `-s`, `--segment PID,PV1` | Only show these segments in detail |
| `--summary` | Segment list and verdict only, no field tables |
| `--raw` | Print the original segment line above each table |
| `-f`, `--field PID-5.1` | Print one value and nothing else |
| `-m`, `--message N` | Only inspect the Nth message in the file |
| `--strict` | Count warnings as failures in the exit code |
| `--color auto\|always\|never` | Colour control (`NO_COLOR` is respected) |
| `--width N` | Wrap at N columns instead of the terminal width |
| `-h`, `--help` | Full help |

## What it checks

**Structure** — the message type in `MSH-9` is matched against the official
message layout (ADT, ORU, ORM/OML, ACK, SIU, MDM, VXU, DFT, BAR, RDE, QRY and
others). A missing required segment is an error; an unexpected or out-of-order
segment is a warning. Site-specific `Z` segments are left alone.

**Required fields** — fields the standard marks as required are errors when
absent. Fields that should be filled in whenever the value is known — patient
address, visit number, observation time — are warnings.

**Data types** — dates and times are checked against the real calendar, so
`19850332` and `20230229` are caught, along with bad timezone offsets,
non-numeric numbers, identifiers with no ID, and locations with no ward.

**Code tables** — coded values are looked up in their HL7 table, so `Q` in the
patient class field is flagged. Unknown codes are warnings, because local code
sets are normal; tables that allow no local values — processing ID, version,
acknowledgement code, yes/no — are errors.

**Consistency** — the cross-field rules that catch real interface bugs:

- the event code in `EVN-1` disagreeing with the trigger in `MSH-9`
- a discharge time earlier than the admission time
- a date of birth in the future, or an implausible age
- a discharge message with no discharge time
- an inpatient with no assigned location
- an observation value that contradicts its declared type (`NM` holding text)
- set IDs on repeating segments that do not count up
- the same patient identifier repeated twice
- accented characters sent with no character set declared in `MSH-18`
- a message control ID too long for the receiving system

Every finding carries a severity, the exact field, the line number in the file
and an explanation of why it matters.

## What it accepts

Real-world message files are messy. `hl7probe` copes with:

- Windows, Unix or classic Mac line endings (`CRLF`, `LF`, `CR`)
- MLLP framing bytes left over from a network capture
- Batch files with `FHS` / `BHS` / `BTS` / `FTS` wrappers
- Several messages in one file, each reported separately
- Non-UTF-8 (latin-1) text, decoded instead of rejected
- Custom delimiters — whatever `MSH-1` and `MSH-2` declare is what is used

## Performance

Messages are parsed, written and dropped one at a time, so peak memory follows
the largest message rather than the size of the file. A batch file is only ever
read once into memory; nothing accumulates as it is reported.

Measured on an Apple M4 (macOS 26.6, rustc 1.97.1, `--release`), best of five
runs, against a file of 50,000 copies of `examples/adt_a01.hl7` — 26 MB, or
about 300,000 segments:

| Command | Time | Peak memory |
| --- | --- | --- |
| `hl7probe -q` (validate only) | 1.25s | 129 MB |
| `hl7probe -f PID-5.1` (one field per message) | 0.57s | 129 MB |
| `hl7probe` (full report) | 3.50s | 129 MB |
| `hl7probe --json` | 6.51s | 129 MB |

At the sizes most runs actually are, this hardly matters: a single message
reports in about 4 ms end to end — most of that being process startup — using
2.4 MB, and 5,000 messages (2.6 MB) validate in 0.13s using 16 MB.

Memory is flat across output modes because none of them hold more than one
parsed message. Before that was true, the same 26 MB file cost 2.2 GB to
validate and 4.6 GB to render as JSON:

| Command | Peak memory before | After |
| --- | --- | --- |
| `hl7probe -q` | 2,234 MB | 129 MB |
| `hl7probe -f PID-5.1` | 2,234 MB | 129 MB |
| `hl7probe` | 2,417 MB | 129 MB |
| `hl7probe --json` | 4,598 MB | 129 MB |

To reproduce:

```sh
python3 -c "open('bulk.hl7','wb').write(open('examples/adt_a01.hl7','rb').read()*50000)"
cargo build --release
/usr/bin/time -l ./target/release/hl7probe -q bulk.hl7   # macOS; use -v on Linux
```

The interactive viewer is the exception: it has to hold every message so you
can page back and forth, so `--tui` over a large batch stays proportional to
the file.

## Building from source

Requires [Rust](https://rustup.rs) 1.88 or newer.

```sh
git clone https://github.com/sudhi001/hl7probe.git
cd hl7probe
cargo build --release      # binary at target/release/hl7probe
cargo test                 # 77 tests
cargo clippy --all-targets
```

The code is organised as:

| File | Responsibility |
| --- | --- |
| `src/parser.rs` | Splitting messages into segments, fields, components |
| `src/spec.rs` | The HL7 dictionary: field names, code tables, message layouts |
| `src/validate.rs` | The checks, one rule per concern |
| `src/view.rs` | The decoded field model both output modes share |
| `src/render.rs` | The printed report |
| `src/tui.rs` | The interactive viewer |
| `src/datetime.rs` | HL7 date and time handling |
| `src/text.rs` | Padding and truncation helpers |
| `src/main.rs` | Command-line interface |

Adding a validation check means writing one `Rule` implementation in
`src/validate.rs` and listing it in `RULES`; nothing else changes.

## Changelog

Release notes live in [CHANGELOG.md](CHANGELOG.md).

## Contributing

Issues and pull requests are welcome. Please make sure `cargo test` and
`cargo clippy --all-targets` pass, and add a test alongside any behaviour
change — the fastest way to describe an HL7 bug is a message that reproduces it.

Note your change under `Unreleased` in [CHANGELOG.md](CHANGELOG.md).

## License

MIT — see [LICENSE](LICENSE).
