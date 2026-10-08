# Proposal: Portfolio mod size split

## Why

`src/portfolio/mod.rs` is 1,019 lines, over the
1,000-physical-line cap enforced by `forge gate`
(`source-file-size`). This is file 5 of the source-file-size grind
per
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9 (verbatim-move rules per §10 decision ledger). File 4
(`portfolio-interest-size-split`) was the first `src/`-side file
and moved the Gate `source-file-size` count 48 → 47; this change
moves it 47 → 46.

## What Changes

- `src/portfolio/mod.rs` (1,019 lines) keeps its module doc,
  constants, validation helpers, record shapes, filter logic,
  and unit tests, and gains `pub mod vocabulary;` plus
  `pub use` re-exports.
- New `src/portfolio/vocabulary.rs` owns the four closed
  vocabularies (`Lifecycle`, `Confidence`, `RelationType`,
  `EvidenceStatus`) with their `impl` blocks copied verbatim —
  the vocabulary half of the module's natural
  vocabulary-vs-validation-vs-records seam.
- Every `pub`/`pub(crate)` path stays byte-identical for callers
  via `pub use` re-exports from `mod.rs`
  (`crate::portfolio::Lifecycle` et al. keep resolving). No
  contract string, CLI output, JSON shape, or behavior changes
  anywhere.
- No other `src/` file touched; no test logic change (imports
  fixed only if a path fails to compile).

## Package Boundary and Split Assessment

One independently verifiable outcome: every file in the
`src/portfolio/` module tree moves under the 1,000-line cap with
zero behavior change. `cargo build` reports 0 errors and the
portfolio suites report identical counts before and after.

| Package | Single outcome | Boundary / contract | Independent oracle | Owned by this change |
|---|---|---|---|---|
| `portfolio-mod-size-split` (**this**) | `src/portfolio/mod.rs` moves under the cap as `mod.rs` + `vocabulary.rs`, vocabulary bodies verbatim, public paths identical | `src/portfolio/` | `cargo build` (0 errors) + portfolio suites identical before/after; Gate `source-file-size` 47 → 46 | yes |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Reusable code / contract | Compatibility gap | Owner | Decision |
|---|---|---|---|---|
| `src/portfolio/mod.rs` (1019) | Vocabularies referenced by `registry::portfolio`, `api::portfolio`, `api::mod`, `main.rs` via `crate::portfolio::{...}` | None — `pub use` re-exports keep every import path resolving; moved items need only `serde::Serialize` in the new file | `src/portfolio/` owns the module | **split in this change** |
| `src/portfolio/interest/activation.rs` (1054), `src/registry/interest/mod.rs` (1047) | Still over the cap | Out of scope — one file per change per parent §9 | their own future changes | **not touched** |
