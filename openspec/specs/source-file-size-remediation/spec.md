# source-file-size-remediation Specification

## Purpose
Bring individual oversized Rust files under the 1000-physical-line cap one file at a time by converting each into a focused directory of submodules, with public symbols, journal columns, routes, catalog rows, CLI arguments, and test names byte-identical to before. The first slice in this repository is `tests/kit_contract.rs`; the change's design records the 55 remaining files as a one-file-per-future-change follow-on roadmap.
## Requirements
### Requirement: Bounded first slice of oversized files is moved under the cap

Forge SHALL bring a bounded first slice of oversized Rust files
under the 1000-physical-line cap by converting `tests/kit_contract.rs`
(2,376 lines) to `tests/kit_contract/` (directory with `main.rs`
plus 8 focused submodules). The crate's `mod ...;` declarations
SHALL NOT change; the only filesystem move SHALL be the
file-to-directory shape of that one test target. Every existing
`cargo test --test kit_contract` target SHALL pass with the same
test count and the same test names as before the change (60
`#[test]` functions, 1 `#[ignore]`).

#### Scenario: Each submodule owns one concern

- **WHEN** an operator inspects `tests/kit_contract/`
- **THEN** `main.rs` holds the file doc, every shared helper,
  the `FeedRestore` RAII struct, and the `mod <area>;`
  declarations, plus the eight submodules
  `registration` / `feed` / `prewiring` / `tokens_and_assets` /
  `tampering` / `classification` / `upgrade` / `manifest` each
  containing the related `#[test]` functions

#### Scenario: Public API, routes, catalog, journal, CLI, and env stay byte-identical

- **WHEN** this change is applied on top of the prior `main`
- **THEN** the `forge --version` output, every `forge <verb> --help`
  output, every `GET /v1/admin/...` envelope, every catalog row,
  every registry/journal row, every CLI exit code, and every
  environment variable name SHALL be byte-identical to before
  the change, and the `--bin forge` catalog-parity test SHALL
  pass at the same row count

#### Scenario: Test count per target is unchanged

- **WHEN** `cargo test --test kit_contract` is run
- **THEN** the target reports 59 passed / 0 failed / 1 ignored
  (the same 60 `#[test]` functions as before the move, with
  `packing_leaves_the_sibling_checkout_byte_identical` still
  `#[ignore]`)

### Requirement: Follow-on roadmap pinned in design.md

Forge SHALL record the remaining 55 oversized Rust files in
`openspec/changes/source-file-size-remediation/design.md` §9
with the natural split axis for each, so a follow-on
OpenSpec change can pick up one file at a time without
re-deciding the strategy. This change SHALL NOT itself
split any of those files.

#### Scenario: Roadmap is present and complete

- **WHEN** the change is archived
- **THEN** `design.md` §9 lists every `src/` and `tests/`
  file that exceeded the 1000-line cap at the time of this
  change's authoring (other than `tests/kit_contract.rs`),
  with the proposed split axis, and none of those files has
  been modified by this change

