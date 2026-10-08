# Proposal: Portfolio interest size split

## Why

`src/portfolio/interest/mod.rs` is 1,002 lines, over the
1,000-physical-line cap enforced by `forge gate`
(`source-file-size`). This is file 4 of the source-file-size grind
per
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9 (verbatim-move rules per §10 decision ledger). The three prior
files were tests-side (`tests/gate_contract.rs`,
`tests/supervised_agent_contract.rs`,
`tests/portfolio_contract.rs`); this is the first `src/`-side
file, so the Gate `source-file-size` count drops by one (48 → 47)
instead of staying flat.

## What Changes

- `src/portfolio/interest/mod.rs` (1,002 lines) keeps its module
  doc, constants, key sets, refusal vocabulary, record shapes,
  helpers, and unit tests, and gains `pub mod vocabulary;` plus
  `pub use` re-exports.
- New `src/portfolio/interest/vocabulary.rs` owns the four closed
  vocabularies (`PrivacyMode`, `Coverage`, `InterestMetric`,
  `SnapshotState`) with their `impl` blocks copied verbatim —
  the types half of the module's natural types-vs-records seam.
- Every `pub`/`pub(crate)` path stays byte-identical for callers
  via `pub use` re-exports from `mod.rs`
  (`crate::portfolio::interest::PrivacyMode` et al. keep
  resolving). No contract string, CLI output, JSON shape, or
  behavior changes anywhere.
- No other `src/` file touched; no test logic change (imports
  fixed only if a path fails to compile).

## Package Boundary and Split Assessment

One independently verifiable outcome: every file in the
`src/portfolio/interest/` module tree moves under the 1,000-line
cap with zero behavior change. `cargo build` reports 0 errors and
the interest contract suites report identical counts before and
after.

| Package | Single outcome | Boundary / contract | Independent oracle | Owned by this change |
|---|---|---|---|---|
| `portfolio-interest-size-split` (**this**) | `src/portfolio/interest/mod.rs` moves under the cap as `mod.rs` + `vocabulary.rs`, vocabulary bodies verbatim, public paths identical | `src/portfolio/interest/` | `cargo build` (0 errors) + interest suites identical before/after; Gate `source-file-size` 48 → 47 | yes |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Reusable code / contract | Compatibility gap | Owner | Decision |
|---|---|---|---|---|
| `src/portfolio/interest/mod.rs` (1002) | Vocabularies referenced by `activation`, `compare`, `validation`, `registry::interest`, `interest_report`, API tests via `crate::portfolio::interest::{...}` | None — `pub use` re-exports keep every import path resolving; moved items need only `serde::Serialize` in the new file | `src/portfolio/interest/` owns the module | **split in this change** |
| `src/portfolio/interest/activation.rs` (1054), `src/registry/interest/mod.rs` (1047), `src/portfolio/mod.rs` (1019) | Still over the cap | Out of scope — one file per change per parent §9 | their own future changes | **not touched** |
