# Tasks: Portfolio interest size split

## 1. BFS — Baseline and package authoring

- [x] Confirm proposal, design, and delta spec agree; a separate implementer could proceed without inventing architecture.
- [x] Record pre-move `cargo build` state and interest-suite counts (unit + contract).
- [x] Add the OpenSpec change directory with proposal, design, and a `portfolio-interest-size-split` delta spec (no TBD Purpose).
- [x] `node scripts/check-openspec-change-names.mjs` PASS; `openspec validate --all --strict --no-interactive` clean with change active.
- [x] Set exactly one `current_spec: portfolio-interest-size-split` line in HANDOFF.md before implementing.

## 2. DFS — Vocabulary extraction (only scoped file)

- [x] `src/portfolio/interest/mod.rs` (1,002 lines) → `mod.rs` + `vocabulary.rs` (`PrivacyMode`, `Coverage`, `InterestMetric`, `SnapshotState` + impls, bodies verbatim); `pub mod vocabulary;` + `pub use` re-exports; every file in the module tree under 1,000 lines.
- [x] `cargo build` 0 errors (pre-existing warnings only); interest suites report identical counts.
- [x] No other `src/` file touched; no test logic change (import fixes only if compilation demands).

## 3. BFS — Cross-surface regression and completeness

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo test --workspace --all-targets` or at minimum the affected contract suites green (counts recorded; pre-existing environmental failures shown identical pre/post via stash comparison if any fail).
- [x] `openspec validate --all --strict --no-interactive` clean; `git diff --check` clean; `node scripts/check-spec-governance.mjs` PASS.

## 4. Verification and archive

- [x] `forge gate --dry-run` renders a plan; `forge gate --timeout-secs 600` runs; verdict recorded in HANDOFF with attribution (source-file-size 48 → 47 expected; no new failure).
- [x] Archive with `openspec archive portfolio-interest-size-split --yes` (no `--skip-specs`); `openspec list` reports no active changes.
- [x] Commit ONLY related paths (explicit); update HANDOFF (delivered+archived entry with evidence, remove pointer when none remain); commit ONLY HANDOFF; stop without pushing.
