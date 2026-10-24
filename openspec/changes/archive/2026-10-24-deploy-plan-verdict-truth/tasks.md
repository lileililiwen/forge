# Tasks: deploy-plan-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the deploy model: `src/deploy/model.rs` (`DeployVerdict`, `verdict_for_report`, `DeployPlan`).
- [x] 1.3 Read the deploy CLI: `src/cli/release.rs` (the deploy plan lane) and `src/cli/publish.rs` (the deploy apply lane).
- [x] 1.4 Read the canonical capabilities: `openspec/specs/adapter-deployment/spec.md` and `openspec/specs/deploy-dry-run-verdict-truth/spec.md`.
- [x] 1.5 Read the existing tests: `tests/deploy_contract.rs`, `tests/deploy_cross_surface.rs`, `tests/deploy_dry_run_verdict_truth_contract.rs`.
- [x] 1.6 Reproduce the defect on the current tree and record the plan's journal row.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the journal state names what the run did — a plan, a blocked plan, a delivery, a rehearsal or a failure.
- [x] 2.2 Decide where the rule lives: in the deploy model, read by the plan lane and the apply lane.
- [x] 2.3 Decide the options for the plan's row and reject the ones that keep a false claim, add vocabulary without truth or lose evidence.
- [x] 2.4 Decide the untouched surfaces: the apply lane's states, the plan's output, the plan's JSON view, the deploy state file, the exit codes, the contract version.

## 3. Implement

- [x] 3.1 `src/deploy/model.rs`: the verdict type gains the plan's arm, and `verdict_for_plan(ready)` is the plan's one rule.
- [x] 3.2 `src/cli/release.rs`: the deploy plan's journal row follows the rule and its detail names the run.
- [x] 3.3 `tests/deploy_plan_verdict_truth_contract.rs`: the CLI contract for a ready plan, a blocked plan and an applied deploy.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A ready deploy plan records `planned` and names the run in the detail.
- [x] 4.4 A blocked deploy plan records `blocked`.
- [x] 4.5 A delivered deploy still records `done`; a rehearsal still records `planned`; a broken deploy still records `failed`.
- [x] 4.6 The plan's output, the plan's JSON view, the deploy state file and the exit codes are unchanged.
- [x] 4.7 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.8 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.9 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 2400` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [x] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [x] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capability's purpose.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

All commands were run from the repository root with
`source /tmp/forge-env.sh` (so `CARGO_TARGET_DIR=/home/paul/code/forge/target`).

| Check | Command | Observed |
|---|---|---|
| Build | `cargo build --workspace` | `Finished \`dev\` profile`, exit 0 |
| Deploy unit tests | `cargo test -p forge --lib deploy::` | 53 passed; 0 failed |
| New contract test | `cargo test --test deploy_plan_verdict_truth_contract` | 4 passed; 0 failed |
| Deploy contracts | `cargo test --test deploy_contract --test deploy_cross_surface --test deploy_dry_run_verdict_truth_contract --test deploy_status_phase_evidence_contract` | 21 + 5 + 3 + 5 passed; 0 failed |
| Full suite | `bash scripts/check-tests.sh` | exit 0; 236 `test result:` lines; 4243 passed; 0 failed; 1 ignored |
| Repository Gate | `forge gate --timeout-secs 2400` | exit 0; 8/8; `observed_at` 2026-10-24T12:24:20Z; revision 0acadcf8bfced4791faef7f623a50bb5ae645e3e |
| Lint | `cargo clippy --workspace --all-targets --all-features` | exit 0; 39 warning lines (the pre-change baseline); the only warning in a touched file is the pre-existing `needless_lifetimes` on `refuse_missing_target_fields` |
| Format | `rustfmt --edition 2021 --check <file>` on each touched file | clean |
| File size | `wc -l` on each touched file | `src/deploy/model.rs` 919, `src/deploy/engine/stages.rs` 744, `src/cli/release.rs` 657, `src/cli/publish.rs` 742, `tests/deploy_plan_verdict_truth_contract.rs` 311 |
| Whitespace | `git diff --check` | no finding |
| Spec validation | `node .../openspec.js validate --all --strict --no-interactive` | `Totals: 160 passed, 0 failed (160 items)` |
| Change names | `node scripts/check-openspec-change-names.mjs` | `PASS` |

The contract test fails on the tree before the change: with
`src/cli/release.rs` reverted, the ready plan's row reads `done` instead
of `planned`, and no detail carries `verdict=`.
