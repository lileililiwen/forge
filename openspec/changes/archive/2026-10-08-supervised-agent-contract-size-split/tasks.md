# Tasks: Supervised agent contract size split

## 1. BFS — Baseline and package authoring

- [x] Confirm proposal, design, and delta spec agree; a separate implementer could proceed without inventing architecture.
- [x] Record pre-move `cargo test --test supervised_agent_contract` counts (29 passed / 0 failed / 0 ignored).
- [x] Add the OpenSpec change directory with proposal, design, and a `supervised-agent-contract-size-split` delta spec (no TBD Purpose).
- [x] `node scripts/check-openspec-change-names.mjs` PASS; `openspec validate --all --strict --no-interactive` clean with change active.
- [x] Set exactly one `current_spec: supervised-agent-contract-size-split` line in HANDOFF.md before implementing.

## 2. DFS — File-to-directory move (only scoped file)

- [x] `tests/supervised_agent_contract.rs` (1,201 lines) → `tests/supervised_agent_contract/` (`main.rs` + 4 submodules: `legacy_session`, `ariadex`, `sisyphusfy`, `native_toolchain`). Helpers and `Fixture` in `main.rs` as `pub(crate)`; each submodule opens with `use super::*;`; every test body verbatim, names unchanged.
- [x] `cargo test --test supervised_agent_contract` post-move reports identical counts (29 passed / 0 failed / 0 ignored).
- [x] No `src/` change; no `mod` change in `src/lib.rs` or `src/main.rs`; no public symbol/route/CLI/env change.

## 3. BFS — Cross-surface regression and completeness

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (pre-existing warnings only).
- [x] No other test target touched; no test added, deleted, renamed, or merged.
- [x] `openspec validate --all --strict --no-interactive` clean; `git diff --check` clean.

## 4. Verification and archive

- [x] `forge gate --dry-run` renders a plan; `forge gate --timeout-secs 600` runs; verdict recorded in HANDOFF with attribution (0 attributable; pre-existing source-file-size fail unchanged since gate evaluates `src/`; adapter-timeout unresolved).
- [x] Archive with `openspec archive supervised-agent-contract-size-split --yes` (no `--skip-specs`); `openspec list` reports no active changes.
- [x] Commit ONLY related paths (explicit); update HANDOFF (delivered+archived entry with evidence, remove pointer when none remain); commit ONLY HANDOFF; stop without pushing.
