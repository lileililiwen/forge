# Tasks: Automatic workspace sync command

## 1. BFS — Baseline and impact coverage

- [ ] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [ ] Read `src/import/mod.rs` (`inspect_import`, `adopt_import`, `derive_project_id`), `src/main.rs` (`Import` command, `Graduation` subcommand pattern, `--format` rendering), `src/registry/mod.rs` (`register`, `check_identity_available`), `src/api/command_catalog.rs` (row builders, pinned count/tests) as the reuse contract.
- [ ] Map each requirement and scenario to its Core function, CLI shape, catalog row and contract-test assertion.
- [ ] Add `tests/workspace_sync_contract.rs` with the design §8 scenarios on a throwaway fixture root.
- [ ] Record the exact toolchain (Rust 2021, `rustc 1.87` floor) and ownership (extend `src/import` + `src/main.rs` + catalog rows; reuse everything else unchanged).

## 2. DFS — Requirement-by-requirement implementation

- [ ] `src/import/mod.rs`: add `WorkspaceSyncEntry` / `WorkspaceSyncReport` / `sync_workspace` (sorted walk, skip rules, per-directory decide → adopt/register/already/skip/fail, counts-only `workspace.sync` journal row).
- [ ] `src/main.rs`: add `Workspace` top-level command + `WorkspaceCommands::Sync`, dispatch and human/JSON rendering with correct exit codes.
- [ ] `src/api/command_catalog.rs`: catalogue `workspace` group + `sync` row as CLI-only; update pinned count 227 → 228 and related pin lists.
- [ ] Implement the delta spec's scenarios alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [ ] Verify single import/register CLI behavior, web management routes, bearer routes, fleet aggregation and journal shapes are unchanged and still tested.
- [ ] Verify the catalog parity/integrity tests, `catalog_contract`, management contract and import unit tests pass.
- [ ] Verify no destructive behavior: vanished directories never prune rows; ambiguous dirs never guessed; failures never abort siblings.
- [ ] Remove any current-change placeholder; confirm no Core/import/registry semantics beyond the new driver were modified.

## 4. Verification

- [ ] `cargo fmt` then `cargo fmt --check` clean.
- [ ] `cargo build` 0 errors.
- [ ] `cargo test` green for: `workspace_sync_contract` (new), `--bin forge` (catalog parity at 228), `catalog_contract`, `forge_web_project_management_contract`, import unit tests (`cargo test --lib import::`), `cargo test --lib api::`. Record the actual pass counts.
- [ ] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [ ] `git diff --check` clean; review newly added files.
- [ ] `forge gate` local run with verdict recorded; no new failure attributable to this change.
- [ ] Archive with `openspec archive forge-workspace-sync --yes` (no `--skip-specs`); promote the canonical spec; update HANDOFF with the evidence and pointer handling.
