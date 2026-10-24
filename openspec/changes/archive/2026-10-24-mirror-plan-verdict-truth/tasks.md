# Tasks: mirror-plan-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the report and the rule: `src/distribution/model.rs` (`MirrorReport`, `healthy_check`).
- [x] 1.3 Read the plan and the apply: `src/distribution/engine.rs` (`plan_mirror`, `apply_mirror`, `render_report_human`).
- [x] 1.4 Read the transport: `src/cli/gitops.rs` (`cmd_mirror`, the journal row, the exit code).
- [x] 1.5 Read the other readers: `src/mcp/methods.rs` (`mcp_mirror_project`), `src/release/engine/stages.rs` (the mirror stage).
- [x] 1.6 Read the canonical capability: `openspec/specs/repository-distribution/spec.md` ("Partial failure and credentials").
- [x] 1.7 Read the existing tests: `tests/distribution_contract.rs`, `tests/distribution_cross_surface.rs`, `src/distribution/engine_tests.rs`.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: a run that attempted no write is a plan; a run that reached every enabled remote is a delivery; anything else is partial.
- [x] 2.2 Decide where the rule lives: in the distribution model, next to the report it judges.
- [x] 2.3 Decide the verdict's names and its wire shape.
- [x] 2.4 Decide that `healthy` is derived from the verdict, and that the exit code follows the verdict rather than `healthy`.
- [x] 2.5 Decide the journal state and the journal detail for a plan.
- [x] 2.6 Decide the untouched surfaces: the per-remote outcomes, the state file, the contract version, the MCP method, the release stage.

## 3. Implement

- [x] 3.1 `src/distribution/model.rs`: `MirrorVerdict`, `verdict_for_outcomes`, the report's `verdict` field, `verdict_check`.
- [x] 3.2 `src/distribution/engine.rs`: `plan_mirror` derives the verdict; `apply_mirror` derives the verdict; `render_report_human` prints the verdict.
- [x] 3.3 `src/cli/gitops.rs`: the journal state, the journal detail and the exit code follow the verdict.
- [x] 3.4 `tests/mirror_plan_verdict_truth_contract.rs`: the CLI contract for a plan, a delivery and a partial delivery.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A plan's verdict is `planned` and the report is not healthy.
- [x] 4.4 A plan's journal row records `planned`, and the command exits 0.
- [x] 4.5 A delivery's verdict is `delivered`, the report is healthy, and the journal records `done`.
- [x] 4.6 A partial delivery's verdict is `partial`, the report is not healthy, and the command exits 1.
- [x] 4.7 The plan and the apply of an unsupported provider both record it as unavailable, and neither claims a delivery.
- [x] 4.8 The per-remote outcomes, the state file and the contract version are unchanged.
- [x] 4.9 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.10 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.11 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 1800` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [ ] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [ ] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capabilities' purposes.
- [ ] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed. The
Gate rows (5.1, 5.2) and the archive rows (6.1 - 6.3) belong to the
owning agent.

| Check | Result | Evidence |
|---|---|---|
| 4.1 `cargo build` | pass | `cargo build --workspace` exited 0 with no warning: `Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.11s`. |
| 4.2 the repository test suite | pass | `bash scripts/check-tests.sh` exited 0 (`SCRIPT_EXIT: 0`); the 230 `test result:` lines sum to **4207 passed; 0 failed; 1 ignored**. |
| 4.2 the distribution unit tests | pass | `cargo test -p forge --lib distribution::` → `test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 1436 filtered out`. |
| 4.2 the new contract test | pass | `cargo test --test mirror_plan_verdict_truth_contract` → `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. |
| 4.2 the existing distribution tests | pass | `cargo test --test distribution_contract --test distribution_cross_surface` → `test result: ok. 12 passed; 0 failed` and `test result: ok. 4 passed; 0 failed`. |
| 4.2 the existing release tests | pass | `cargo test --test release_contract --test release_cross_surface` → `test result: ok. 10 passed; 0 failed` and `test result: ok. 4 passed; 0 failed`. |
| 4.2 a pre-existing flake in the checker contract | recorded, unrelated | One earlier run of the suite on this tree failed `tests/checker_contract.rs::format_json_does_not_alter_checker_stdout` with `left: {"schema":"forge-checker/0.1.0",2026-10-24T02:38:12Z","alerts":[]}` and `right: {"schema":"forge-checker/0.1.0",2026-10-24T02:38:13Z","alerts":[]}`. The test compares the human and the JSON run of the same command after stripping the `"generated_at":"` key prefix but not its value, so it fails whenever the two runs straddle a second boundary. It is a pre-existing timing flake in a test this change does not touch; it passed on the re-run (`cargo test --test checker_contract` → `14 passed; 0 failed`), it passed in the worker's own suite run, and it passed in the Gate's `tests` check. The remedy for a future occurrence is to compare the two documents without the `generated_at` value. |
| 4.3 a plan's verdict | pass | `dry_run_mirror_is_planned_and_journals_a_planned_row` asserts the JSON report's `verdict` is `planned`, `dry_run` is `true`, `healthy` is `false` and the note contains `plan only`. The unit test `plan_report_is_planned_and_never_healthy` asserts `report.verdict == MirrorVerdict::Planned` and `!report.healthy()`. |
| 4.4 the plan's journal row | pass | The same contract test reads the `mirror` row of the registry's `operations` table and asserts its state is `planned` and its detail carries `verdict=planned` and `healthy=false`; the command's exit code is `0`. The human render of the same run carries `verdict: planned` and `summary: plan only`. |
| 4.5 a delivery's verdict | pass | `a_delivered_mirror_is_delivered` (a real bare repo for the primary and for the mirror) asserts `verdict` is `delivered`, `healthy` is `true`, the exit code is `0` and the journal row's state is `done`. |
| 4.6 a partial delivery's verdict | pass | `a_partial_mirror_is_partial` (the primary is a real bare repo, the mirror remote is absent) asserts `verdict` is `partial`, `healthy` is `false`, the exit code is `1` and the journal row's state is `partial`. |
| 4.7 the unsupported provider | pass | `an_unsupported_provider_is_unavailable_in_both_the_plan_and_the_apply` asserts the plan's `gitlab` outcome is `unavailable` with the verdict `planned` and `healthy` false, and the apply's `gitlab` outcome is `unavailable` with the verdict `partial`, `healthy` false and exit code `1`. The apply's `github` outcome is `delivered`, so the primary was written and the planned provider was not. |
| 4.8 the untouched evidence | pass | `git diff` touches no outcome construction, no state-file write and no `DISTRIBUTION_CONTRACT_VERSION`; the report keeps every existing field and gains only `verdict`. `tests/distribution_contract.rs` and `tests/distribution_cross_surface.rs` stay green (12 + 4 passed). |
| 4.9 rustfmt | pass | `rustfmt --edition 2021 --check` is clean on all five touched Rust files. `cargo fmt --all` was never run, and no pre-existing rustfmt-dirty file was touched. |
| 4.10 clippy | pass | `cargo clippy --workspace --all-targets --all-features` exited 0 with 39 warnings, the same count as the pre-change baseline, and no warning points at a touched file. |
| 4.11 the source file line counts | pass | Every touched file stays under the 1000 physical line limit: `src/distribution/model.rs` 304, `src/distribution/engine.rs` 679, `src/distribution/engine_tests.rs` 593, `src/cli/gitops.rs` 332, `tests/mirror_plan_verdict_truth_contract.rs` 379. |
| 5.1 `forge gate --dry-run` | pass | `./target/debug/forge gate --dry-run` resolved the same 8 required checks, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`, exit 0. |
| 5.2 `forge gate --timeout-secs 2400` | pass | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-24T03:59:51Z"` on revision `be7adb960c0f33809ca15ba119dbd5335ec6a585` (the parent commit — the implementation is uncommitted while the Gate runs), with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. |
| 5.3 strict validation | pass | `openspec validate --all --strict --no-interactive` → `Totals: 154 passed, 0 failed (154 items)`, including `✓ change/mirror-plan-verdict-truth`. |
| 5.4 the change names | pass | `node scripts/check-openspec-change-names.mjs` → `check-openspec-change-names: PASS`. |
| 5.5 `git diff --check` | pass | `git diff --check` → no finding. |

### Judgement calls

- **The exit code follows the verdict, not `healthy`.** `Delivered` and
  `Planned` return `Ok(output)`; `Partial` keeps today's behaviour
  exactly: the per-remote evidence is printed on stdout and the
  command returns `ForgeError::DistributionInvalid`, so it exits 1. A
  plan is a successful plan, not a failure.
- **`healthy` stays a field.** It is now derived from the verdict
  (`true` only for `delivered`), so every existing reader keeps
  working; `MirrorReport::healthy()` still returns the field. The
  report's `verdict_check()` is the public form of the one rule, so a
  reader can re-derive the verdict from the report's own evidence.
- **`apply_mirror` derives the verdict from the outcomes it collected**
  (`verdict_for_outcomes(false, &outcomes)`), not from the state file
  and not from a second probe. The per-remote outcomes, their notes,
  their evidence and their recovery lines are untouched.
