# Tasks: Source file size remediation

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read every oversized file in `src/` and `tests/` to know what splits are natural.
- [x] Map each requirement and scenario to its target file and the test target that proves the move.
- [x] Add the OpenSpec change directory with proposal, design and a `source-file-size-remediation` delta spec.
- [x] Pin the follow-on roadmap (one file per future change) in `design.md` §9.

## 2. DFS — File-by-file split (this change's first slice)

- [x] `tests/kit_contract.rs` (2376 lines) → `tests/kit_contract/` (directory with `main.rs` plus 8 submodules: `registration`, `feed`, `prewiring`, `tokens_and_assets`, `tampering`, `classification`, `upgrade`, `manifest`). Helpers in `main.rs`, tests grouped by area into submodules. `cargo test --test kit_contract` stays at 59 passed / 1 ignored (same as before the move).

## 3. BFS — Cross-surface regression and completeness

- [x] Every other test target stays green at the same pass count.
- [x] No `mod ...;` declaration in `src/lib.rs` or `src/main.rs` changes.
- [x] No public symbol, journal column, route, CLI argument, catalog row, env var, or `--version` output changes.
- [x] The follow-on roadmap is preserved in `design.md` §9 and no follow-on file is touched.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] `cargo test --test kit_contract` 59 passed / 0 failed / 1 ignored (same count as before).
- [x] `cargo test --bin forge` (catalog parity), `forge_web_command_catalog_contract`, `forge_web_project_management_contract`, `forge_admin_api_contract`, `forge_web_fleet_contract`, `portal_ui_contract` all green.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [x] `forge gate` local run with verdict recorded; the `source-file-size` failing count drops by 1 (the converted `tests/kit_contract.rs` is no longer evaluated) with no new failure attributable to this change.
- [x] Archive with `openspec archive source-file-size-remediation --yes` (no `--skip-specs`); promote the canonical spec; update HANDOFF with the evidence and pointer handling.
