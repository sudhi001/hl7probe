# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- The lint policy moved into a `[lints]` table in `Cargo.toml`, so a local
  `cargo clippy` enforces exactly what CI does: `clippy::pedantic`, plus
  `unsafe_code = "forbid"` and `clippy::unwrap_used` for the code that handles
  input the tool does not control. Test modules opt out of the latter.
- Roughly 180 pedantic findings cleared: inline format arguments, `write!`
  instead of `push_str(&format!(..))`, `const fn` where the body allows it,
  and checked conversions in place of every `as` cast that could truncate or
  change sign. Output is byte-identical across the example messages and 3600
  mutated ones.
- The two `unwrap()` calls left in non-test code became `let ... else` and
  `filter_map`; both were already guarded, so nothing changes at runtime.

### Added

- Unit tests for the exit-status contract, field-path parsing, the latin-1
  input fallback and file labelling.
- CI builds and tests on the declared minimum Rust version, builds with
  `--locked`, and fails on rustdoc warnings.

## [0.2.2] - 2026-08-18

### Changed

- The release binary is about 17% smaller — 1.2 MB to 1.0 MB on Apple silicon —
  from building in one codegen unit and dropping unwinding tables. Parsing
  speed is unchanged: 5000 messages in 0.15s either way. Panic hooks still run
  under `panic = "abort"`, so the interactive viewer restores the terminal if
  it ever crashes.

## [0.2.1] - 2026-08-18

### Fixed

- The declared minimum Rust version was 1.74, which no toolchain could honour:
  `clap` needs 1.85 and `darling`, pulled in through `ratatui`, needs 1.88. It
  now says 1.88, verified by building and testing with that toolchain.
- The published crate excluded `examples/` while shipping the integration tests
  that read them, so `cargo test` failed for anyone who downloaded it.
- README images use absolute URLs so they render on crates.io.

### Added

- Published on [crates.io](https://crates.io/crates/hl7probe):
  `cargo install hl7probe`.

## [0.2.0] - 2026-08-18

### Changed

- **The command is now `hl7probe`, not `hl7test`.** One name for the project,
  the repository, the Homebrew formula and the binary. Update any script that
  calls `hl7test`; nothing else about the interface changed.
- Release archives are named `hl7probe-<version>-<target>` to match.

## [0.1.2] - 2026-08-18

### Added

- Windows binaries. Releases carry
  `hl7test-<version>-x86_64-pc-windows-msvc.zip` alongside the macOS and Linux
  archives. (Archives up to 0.1.2 carry the old `hl7test` name; 0.2.0 onwards
  are `hl7probe-<version>-<target>`.)

### Changed

- Every release binary except the cross-compiled Intel macOS one is started on
  its own runner before being packaged.

## [0.1.1] - 2026-08-18

### Fixed

- The Linux binaries are now static musl builds. The 0.1.0 ones were linked
  against the build runner's glibc 2.39 and would not start on Debian 12,
  Ubuntu 22.04, RHEL 8 or 9, or Amazon Linux, which report
  `version 'GLIBC_2.39' not found`. The new binaries have no libc dependency
  and run on any Linux, including Alpine.
- Release checksum files record only the archive name, so `shasum -a 256 -c`
  works in the directory a user downloads into.
- The Intel macOS binary is cross-compiled on an Apple silicon runner, which no
  longer leaves a release waiting on a scarce Intel runner.
- The release job reports why creating a release failed instead of silently
  trying to upload to a release that does not exist.

### Added

- A demo GIF and screenshots in the README, generated from real output by
  `docs/tools/render_media.py`.

## [0.1.0] - 2026-08-17

First release.

### Added

- `hl7test` command that decodes an HL7 v2 message into named fields and
  reports what a receiving system would reject.
- Parser for segments, fields, repetitions, components and subcomponents, with
  support for custom delimiters, escape sequences, CR/LF/CRLF line endings,
  MLLP framing bytes, HL7 batch wrappers and several messages per file.
- Dictionary of segment fields, HL7 code tables and abstract message structures
  covering ADT, ORU, ORM/OML, ACK, SIU, MDM, VXU, DFT, BAR, RDE and QRY.
- Validation across five groups: structure, required fields, data types, code
  tables and cross-field consistency.
- Plain-language decoding of names, dates, coded values, identifiers,
  addresses and patient locations.
- Interactive viewer (`--tui`) with segment, field and validation panels.
- JSON reports (`--json`), single-value queries (`--field`), quiet mode
  (`--quiet`), segment filters (`--segment`) and `--strict` exit codes.
- Homebrew formula and prebuilt binaries for macOS and Linux.

[Unreleased]: https://github.com/sudhi001/hl7probe/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/sudhi001/hl7probe/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/sudhi001/hl7probe/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/sudhi001/hl7probe/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/sudhi001/hl7probe/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/sudhi001/hl7probe/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/sudhi001/hl7probe/releases/tag/v0.1.0
