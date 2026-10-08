# portfolio-interest-size-split (delta)

## Purpose

Keep the portfolio interest module under the file-size cap by
extracting its four closed vocabularies into a submodule with
every body verbatim and every public path identical for callers.

## ADDED Requirements

### Requirement: Interest vocabularies move to a submodule with paths stable

Forge SHALL move the four closed vocabularies (`PrivacyMode`,
`Coverage`, `InterestMetric`, `SnapshotState` with their `impl`
blocks) from `src/portfolio/interest/mod.rs` (1,002 lines) to
`src/portfolio/interest/vocabulary.rs`, with `mod.rs` declaring
`pub mod vocabulary;` and re-exporting every moved name via
`pub use` so that each `crate::portfolio::interest::<name>` path
resolves exactly as before. Every moved body SHALL be copied
verbatim with no cleanup and no renames. Every file in the
`src/portfolio/interest/` module tree SHALL be under 1,000 lines.

#### Scenario: Each vocabulary lives in exactly one place

- **WHEN** an operator inspects `src/portfolio/interest/`
- **THEN** `vocabulary.rs` holds the four vocabulary enums and
  their `impl` blocks, and `mod.rs` holds the module doc,
  constants, key sets, refusal vocabulary, record shapes,
  helpers, and unit tests plus the `pub mod` / `pub use` lines

#### Scenario: Public API, contracts, CLI, and JSON stay byte-identical

- **WHEN** this change is applied on top of the prior `main`
- **THEN** every `pub`/`pub(crate)` path, re-export, contract
  string (`forge-portfolio-interest/0.1.0`,
  `forge-portfolio-activation/0.1.0`), refusal code, metric
  label, CLI output, JSON shape, and behavior SHALL be
  byte-identical to before the change, and `cargo build`
  reports 0 errors

#### Scenario: Interest tests are unchanged in count and outcome

- **WHEN** the interest unit tests and portfolio/interest
  contract suites are run
- **THEN** they report identical passed / failed / ignored
  counts before and after the move, with no test added,
  deleted, renamed, or re-logicked
