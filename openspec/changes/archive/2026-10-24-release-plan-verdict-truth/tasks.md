# Tasks: release-plan-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the release model: `src/release/model.rs`.
- [x] 1.3 Read the release CLI: `src/cli/release.rs` (`cmd_release_prepare`, `cmd_release_apply`).
- [x] 1.4 Read the canonical capabilities: `openspec/specs/release-publishing/spec.md` and `openspec/specs/release-dry-run-evidence-truth/spec.md`.
- [x] 1.5 Read the existing tests: `tests/release_contract.rs`, `tests/release_cross_surface.rs`.
- [x] 1.6 Reproduce the defect on the current tree and record the preparation's journal row.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the journal state names what the run did — a plan, a blocked plan, an applied release, an incomplete applied release.
- [x] 2.2 Decide where the rule lives: in the release model, read by both writers.
- [x] 2.3 Decide the options for the preparation's row and reject the ones that keep a false claim or add vocabulary without truth.
- [x] 2.4 Decide the untouched surfaces: the apply lane's states, the prepare's output, the release state file, the API's plan route, the exit codes, the contract version.

## 3. Implement

- [x] 3.1 `src/release/model.rs`: `ReleaseRun` with its journal state and its label.
- [x] 3.2 `src/cli/release.rs`: the prepare's journal row follows the rule; the apply's journal row reads the same rule; both details name the run.
- [x] 3.3 `tests/release_plan_verdict_truth_contract.rs`: the CLI contract for a ready plan, a blocked plan and an applied release.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A ready preparation records `planned` and names the run in the detail.
- [x] 4.4 A blocked preparation records `blocked`.
- [x] 4.5 A preparation run as a dry run records `planned`.
- [x] 4.6 An applied release still records `done`; a dry run of an apply still records `planned`; an incomplete release still records `partial`.
- [x] 4.7 The prepare's output, the release state file, the API's plan route and the exit codes are unchanged.
- [x] 4.8 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.9 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.10 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 2400` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [x] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [x] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capabilities' purposes.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

All commands were run from the repository root with
`source /tmp/forge-env.sh` (so `CARGO_TARGET_DIR=/home/paul/code/forge/target`).

| Check | Command | Observed |
|---|---|---|
| Build | `cargo build --workspace` | `Finished \`dev\` profile`, exit 0 |
| Release unit tests | `cargo test -p forge --lib release::` | 40 passed; 0 failed |
| New contract test | `cargo test --test release_plan_verdict_truth_contract` | 5 passed; 0 failed |
| Release contracts | `cargo test --test release_contract --test release_cross_surface --test release_dry_run_truth_contract --test release_gate_mandatory_contract --test release_driftwatch_check_truth_contract --test project_changelog_bootstrap_contract --test forge_web_project_release_contract` | all passed; 0 failed |
| Full suite | `bash scripts/check-tests.sh` | exit 0; 235 `test result:` lines; 4238 passed; 0 failed; 1 ignored |
| Repository Gate | `forge gate --timeout-secs 2400` | exit 0; 8/8; `observed_at` 2026-10-24T11:33:33Z; revision e5f338f29e8c2024921be2c64ee8b3d0e8f3fc09 |
| Lint | `cargo clippy --workspace --all-targets --all-features` | exit 0; 39 warning lines (the pre-change baseline); no warning names a touched file |
| Format | `rustfmt --edition 2021 --check <file>` on each touched file | clean |
| File size | `wc -l` on each touched file | `src/release/model.rs` 610, `src/cli/release.rs` 650, `tests/release_plan_verdict_truth_contract.rs` 404 |
| Whitespace | `git diff --check` | no finding |
| Spec validation | `node .../openspec.js validate --all --strict --no-interactive` | `Totals: 159 passed, 0 failed (159 items)` |
| Change names | `node scripts/check-openspec-change-names.mjs` | `PASS` |

The contract test fails on the tree before the change: with
`src/cli/release.rs` reverted, all five cases fail — the ready
preparation's row reads `done` instead of `planned`, and no detail
carries `verdict=`.
