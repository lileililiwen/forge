# portfolio-mod-size-split Specification

## Purpose
Keep the portfolio module under the 1,000-line file-size cap by extracting its four closed vocabularies (`Lifecycle`, `Confidence`, `RelationType`, `EvidenceStatus`) into `src/portfolio/vocabulary.rs` with every body verbatim and every `crate::portfolio::<name>` path identical for callers.
## ADDED Requirements
### Requirement: Portfolio vocabularies move to a submodule with paths stable

Forge SHALL move the four closed vocabularies (`Lifecycle`,
`Confidence`, `RelationType`, `EvidenceStatus` with their `impl`
blocks) from `src/portfolio/mod.rs` (1,019 lines) to
`src/portfolio/vocabulary.rs`, with `mod.rs` declaring
`pub mod vocabulary;` and re-exporting every moved name via
`pub use` so that each `crate::portfolio::<name>` path resolves
exactly as before. Every moved body SHALL be copied verbatim
with no cleanup and no renames. Every file in the
`src/portfolio/` module tree SHALL be under 1,000 lines.

#### Scenario: Each vocabulary lives in exactly one place

- **WHEN** an operator inspects `src/portfolio/`
- **THEN** `vocabulary.rs` holds the four vocabulary enums and
  their `impl` blocks, and `mod.rs` holds the module doc,
  constants, validation helpers, record shapes, filter logic,
  and unit tests plus the `pub mod` / `pub use` lines

#### Scenario: Public API, contracts, CLI, and JSON stay byte-identical

- **WHEN** this change is applied on top of the prior `main`
- **THEN** every `pub`/`pub(crate)` path, re-export, contract
  string (`forge-portfolio/0.1.0`), validation message, CLI
  output, JSON shape, and behavior SHALL be byte-identical to
  before the change, and `cargo build` reports 0 errors

#### Scenario: Portfolio tests are unchanged in count and outcome

- **WHEN** the portfolio unit tests and portfolio contract suite
  are run
- **THEN** they report identical passed / failed / ignored
  counts before and after the move, with no test added,
  deleted, renamed, or re-logicked
