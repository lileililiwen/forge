# Tasks: api-operation-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the boundary: `src/api/server.rs` (`run_with_operation`, `pending_or_done_response`) and every one of its fifteen call sites.
- [x] 1.3 Read the two plan routes: `src/api/handlers_portfolio.rs` (the deploy route) and `src/api/handlers_changes.rs` (the upgrade route).
- [x] 1.4 Read the rules that already exist: `src/deploy/model.rs` (`DeployVerdict`, `journal_state`, `verdict_for_report`, `verdict_for_plan`), `src/cli/release.rs` (the command's plan lane), `src/upgrade/engine.rs` (`plan_upgrade`).
- [x] 1.5 Read the canonical capabilities: `openspec/specs/core-http-api/spec.md`, `openspec/specs/deploy-plan-verdict-truth/spec.md`, `openspec/specs/deploy-dry-run-verdict-truth/spec.md`.
- [x] 1.6 Read the existing tests: `tests/api_agent_transition_verdict_truth_contract.rs`, `tests/forge_web_project_deployment_contract.rs`, `tests/api_cross_surface.rs`.
- [x] 1.7 Reproduce the defect on the current tree and record the journal rows it wrote.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: an operation row's state names what the operation did — completed, planned, blocked or failed.
- [x] 2.2 Decide where the rule lives: one new module, `src/api/operation.rs`, next to the boundary that writes the row.
- [x] 2.3 Decide the options and reject the second helper, the flag parameter, the value-derived outcome and the hand-written state.
- [x] 2.4 Decide the untouched surfaces: the CLI lanes, the deploy verdict rule, the registry journal, the failure path of the boundary, the HTTP envelopes.

## 3. Implement

- [x] 3.1 `src/api/operation.rs`: the `OperationOutcome` rule with its state, its detail and the deploy verdict mapping.
- [x] 3.2 `src/api/server.rs`: the boundary records the outcome the closure earned; the replay helper is renamed.
- [x] 3.3 `src/api/handlers_portfolio.rs`: the deploy route states the plan's or the report's outcome.
- [x] 3.4 `src/api/handlers_changes.rs`: the upgrade route states its plan's outcome; the other routes state the completed operation.
- [x] 3.5 `src/api/admin/deploy.rs`: the apply states the report's verdict.
- [x] 3.6 `src/api/project_management.rs`, `src/api/workbench.rs`, `src/api/admin/release.rs`: the remaining routes state the completed operation.
- [x] 3.7 `tests/api_operation_verdict_truth_contract.rs`: the plan, the blocked plan, the applied request, the failed run and the upgrade plan.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A planned deployment is journalled as a planned operation, never as a completed one.
- [x] 4.4 A planned upgrade is journalled as a planned operation.
- [x] 4.5 A blocked plan is journalled as a blocked operation.
- [x] 4.6 An applied request still journals a completed operation.
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

- [x] 6.1 Archive without `--skip-specs`, so the added capability is promoted and the modified one is updated.
- [x] 6.2 Write the added capability's purpose by hand and strip the archive's trailing blank lines.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

### The reproduced defect

The boundary finalised every successful request as a completed operation:

```rust
// src/api/server.rs, before
Ok((value, op_id, project_id)) => {
    let detail = format!("{kind} completed");
    let _ = registry.finalize_operation(op_id, "done", &detail);
```

Two project routes run a plan when the request asks for one, so the row
claimed work the response body itself says did not happen. With the
source changes stashed and the new contract test in place, four of the
six cases fail:

```
---- a_dry_run_deployment_journals_a_planned_operation stdout ----
assertion `left == right` failed: a plan is not a completed operation; detail=api.deploy completed
  left: "done"
 right: "planned"

---- a_blocked_deploy_plan_journals_a_blocked_operation stdout ----
assertion `left == right` failed: detail=api.deploy completed
  left: "done"
 right: "blocked"

---- a_dry_run_upgrade_journals_a_planned_operation stdout ----
assertion `left == right` failed: a plan is not a completed upgrade; detail=api.upgrade completed
  left: "done"
 right: "planned"

---- a_failed_deploy_run_is_not_a_completed_operation stdout ----
assertion `left == right` failed: a run that did not finish is not a completed operation; detail=api.deploy completed
  left: "done"
 right: "failed"

test result: FAILED. 2 passed; 4 failed; 0 ignored
```

The two cases that pass on the pre-change tree are the applied feature
request and the delivered deploy: both really did the work, and both
keep their completed operation.

The repository already had the rule that says what those rows should
say. `DeployVerdict::journal_state` maps `Planned` to `planned`, and
the command's plan lane already reads it:

```rust
// src/cli/release.rs
let verdict = forge::deploy::verdict_for_plan(plan.ready);
registry.record_operation("deploy", &project_id, verdict.journal_state(), &detail);
```

### The change

| Surface | Before | After |
|---|---|---|
| `src/api/operation.rs` | did not exist | the one rule: `OperationOutcome` with its state, its detail and the deploy verdict mapping |
| `src/api/server.rs` boundary | every successful closure became `done` | the row's state and detail come from the outcome the closure earned |
| `src/api/server.rs` replay helper | `pending_or_done_response` | `replay_response`, because the old name stated something false |
| `src/api/handlers_portfolio.rs` deploy route | a plan was `done` | a plan reads `verdict_for_plan`, an apply reads the report's verdict |
| `src/api/handlers_changes.rs` upgrade route | a plan was `done` | a plan is `planned` |
| `src/api/admin/deploy.rs` apply route | every apply was `done` | the apply reads the report's verdict |
| The other eleven routes | `done` | `done`, stated by the route |

### The checks

| Check | Result | Evidence |
|---|---|---|
| The reproduced defect | reproduced before the change | With the source changes stashed and the new contract test in place, four of its six cases fail: the planned deployment (`left: "done"`, `right: "planned"`, `detail=api.deploy completed`), the blocked plan (`right: "blocked"`), the planned upgrade (`right: "planned"`, `detail=api.upgrade completed`) and the failed run (`right: "failed"`). `test result: FAILED. 2 passed; 4 failed`. |
| A plan is never a completed operation | PASS | `a_dry_run_deployment_journals_a_planned_operation`: the body is the plan (`deploy.ready == true`), the row's state is `planned` and its detail is `api.deploy planned`. |
| A blocked plan stays blocked | PASS | `a_blocked_deploy_plan_journals_a_blocked_operation`: without the Gate evidence the shared delivery rule refuses the revision, the body says `deploy.ready == false`, and the row's state is `blocked`. |
| A planned upgrade is not a completed upgrade | PASS | `a_dry_run_upgrade_journals_a_planned_operation`: the body is the plan and the row's state is `planned`. |
| An applied request keeps its completed operation | PASS | `an_applied_request_still_journals_a_completed_operation`: the `api.add_feature` row is `done` with the detail `api.add_feature completed`. |
| A run that did not finish is not a completed operation | PASS | `a_failed_deploy_run_is_not_a_completed_operation`: the report's own verdict is `failed` and the row's state is `failed`, not `done`. |
| A delivered deploy keeps its completed operation | PASS | `a_delivered_deploy_run_journals_a_completed_operation`: the report's verdict is `delivered` and the row's state is `done`. |
| The untouched surfaces | PASS | The CLI lanes, the deploy verdict rule, the registry journal, the boundary's failure path and every HTTP envelope are unchanged. The existing suites pass unchanged. |
| Repository test suite | PASS | `bash scripts/check-tests.sh` exited 0; the 248 `test result:` lines sum to **4299 passed; 0 failed; 1 ignored**. The pre-change baseline was 247 lines and 4293 passed, so this change adds exactly the 6 tests it declares and one new test binary. |
| Repository Gate | PASS, 8/8 | Exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-25T14:05:17Z"` on revision `a75b9b2477efdf5e057f4384b468692c19b89d36` (the parent commit — the implementation is uncommitted while the Gate runs), with all eight required checks — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` — in state `pass`. The rehearsal before it (`forge gate --dry-run`) resolved the same 8 required checks, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. |
| Lint and formatting | PASS | `cargo clippy --workspace --all-targets --all-features` exited 0 with 39 lines starting with `warning`, the same count as the pre-change baseline, and no warning names a touched file. `rustfmt --edition 2021 --check` is clean on every touched Rust file, and `cargo fmt --all` was never run. No pre-existing rustfmt-dirty file was touched. Every touched file stays under the 1000 physical line limit. |
