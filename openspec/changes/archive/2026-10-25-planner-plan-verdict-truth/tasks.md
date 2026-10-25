# Tasks: planner-plan-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the lanes: `src/cli/intent.rs` (the resolve arm), `src/planner/apply.rs` (the component step), `src/planner/model.rs` (`AppliedStep`).
- [x] 1.3 Read the shared rule: `src/core/resolve.rs` (`ResolveRun`).
- [x] 1.4 Read the canonical capabilities: `openspec/specs/resolve-plan-verdict-truth/spec.md`, `openspec/specs/validated-intent-planner/spec.md`.
- [x] 1.5 Read the existing tests: `tests/planner_contract.rs`, `tests/planner_cross_surface.rs`.
- [x] 1.6 Reproduce the defect on the current tree and record the journal row it wrote.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the planner names the action it performed — a resolve is a planned run, a resolved component is a resolved component.
- [x] 2.2 Decide where the resolve rule lives: the shared `ResolveRun` rule the planner now reads.
- [x] 2.3 Decide the options and reject the ones that keep a completed resolution on the record, copy the rule into the lane or claim a verification the planner did not perform.
- [x] 2.4 Decide the untouched surfaces: the plan, the receipt, the apply gate, the other step kinds, the other lanes.

## 3. Implement

- [x] 3.1 `src/cli/intent.rs`: the resolve lane reads the shared rule; the detail names the run.
- [x] 3.2 `src/planner/apply.rs`: the component step names the action it performed.
- [x] 3.3 `tests/planner_plan_verdict_truth_contract.rs`: the row's state, the row's detail and the applied step's status.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A resolve journals `planned` and names the run.
- [x] 4.4 An applied component step reports `resolved`, never `verified`.
- [x] 4.5 The plan, the receipt, the apply gate, the other step kinds and the other lanes are unchanged.
- [x] 4.6 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.7 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.8 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 2400` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [x] 6.1 Archive without `--skip-specs`, so the modified capabilities are promoted.
- [x] 6.2 Update the modified capabilities' purposes.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

### The reproduced defect

```
$ forge --registry /tmp/plan-probe/registry.db intent resolve --action create_project --profile rust-web
$ sqlite3 /tmp/plan-probe/registry.db "SELECT kind, project_id, state, detail FROM operations"
('planner', 'rust-web-1ae2bc11-a90ac786', 'done', 'resolve: deterministic assembly plan for create_project: 3 step(s), 0 unresolved, profile rust-web@0.1.0')
```

The row said `done` for a plan that `forge intent apply` — a separate,
confirm-gated command — never applied. The state came from a private
copy of the rule in a third lane: `src/cli/intent.rs` wrote `"done"`
while `src/core/resolve.rs` already owned `ResolveRun::for_steps`.

The second defect was in the apply lane: the component step's status
was `verified` although the step only called `resolve_components` and
its own note said the planner "did not auto-qualify it".

### The change

| Surface | Before | After |
|---|---|---|
| `src/cli/intent.rs` resolve row | `done` | `planned`, by `ResolveRun::for_steps` |
| `src/cli/intent.rs` resolve detail | `resolve: <plan note>` | `resolve: verdict=planned <plan note>` |
| `src/cli/intent.rs` resolve output | no run named | the JSON envelope carries the note; the human render ends with it |
| `src/planner/apply.rs` component step | `verified` | `resolved` |
| `src/core/resolve.rs` module doc | two lanes | three lanes |
| `src/planner/mod.rs` module doc | `done` / `rejected` / `partial` | `planned` / `done` / `rejected` / `partial` |

### The checks

| Check | Result | Evidence |
|---|---|---|
| The reproduced defect | reproduced before the change | The row above: `('planner', 'rust-web-1ae2bc11-a90ac786', 'done', 'resolve: deterministic assembly plan ...')` |
| A resolve journals a planned run | PASS | `an_intent_resolve_journals_a_planned_run_and_names_it`: the row's state is `planned`, its detail names `verdict=planned`, the row names the plan id, and the command's JSON note names the run. |
| A resolve is never a completed resolution | PASS | `an_intent_resolve_is_never_recorded_as_a_completed_resolution`: no planner row reads `done`, the only artifact is the plan receipt, and no feature directory exists. |
| A resolved component is not a verified install | PASS | `an_applied_component_step_reports_resolved_and_never_verified`: the applied step's status is `resolved`, never `verified` or `installed`, and the note still says the planner did not auto-qualify it. |
| One rule, every resolution writer | PASS | `every_resolution_writer_reads_the_same_rule`: the component, UI pattern and intent lanes each journal `planned` with `verdict=planned`. |
| The contract test fails without the change | PASS | With `src/cli/intent.rs` and `src/planner/apply.rs` reverted to `HEAD`, all four cases fail: the resolve row reads `done` instead of `planned`, the completed-resolution case finds that `done` row, the applied component step reads `verified` instead of `resolved`, and the shared-rule case reads `done` for the planner lane. With the change restored, all four pass. |
| The plan, the receipt and the apply gate | unchanged | `git diff` touches no line of `src/planner/resolve.rs`, `src/planner/model.rs`, `src/planner/render.rs` or `src/planner/apply.rs`'s other step kinds. `tests/planner_contract.rs` (11 tests) and `tests/planner_cross_surface.rs` (7 tests) pass unchanged, including the cross-surface test that reads a completed planner apply as `done`. |
| Repository test suite | PASS | `bash scripts/check-tests.sh` exited 0; the 243 `test result:` lines sum to **4278 passed; 0 failed; 1 ignored**. The pre-change baseline was 242 lines and 4274 passed, so this change adds exactly the 4 tests it declares and one new test binary. |
| Repository Gate | PASS, 8/8 | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-25T09:28:21Z"` on revision `5ce3748ecb1c3d4d1ae219bbe94a507ea7f4fc89` (the parent commit — the implementation is uncommitted while the Gate runs), with all eight required checks — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` — in state `pass`. The `source-file-size` check evaluated 460 of 460 candidate files under the 1,000-line limit. The rehearsal before it (`forge gate --dry-run`) resolved the same 8 required checks, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. The advisory database's fetch was resolved to the local mirror of the same cached database (`/home/paul/.cache/advisory-db-mirror.git`, commit `640bf5a9`) through `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_0=url.file:///home/paul/.cache/advisory-db-mirror.git.insteadOf` / `GIT_CONFIG_VALUE_0=https://github.com/RustSec/advisory-db.git`, and `CARGO_HTTP_PROXY=http://127.0.0.1:1` made the unreachable crates.io index update fail fast instead of waiting for the TCP timeout. Both settings applied to that one Gate process only. |
| OpenSpec, change names, whitespace | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 167 passed, 0 failed (167 items)`; `node scripts/check-openspec-change-names.mjs` → `PASS`; `git diff --check` and `git diff --cached --check` → no finding. Archived without `--skip-specs`; the modified capabilities were promoted, the archive's `TBD` Purpose was replaced by hand with the real one, and no canonical scenario name was dropped. |
| Lint and formatting | PASS | `cargo clippy --workspace --all-targets --all-features` exited 0 with 39 lines starting with `warning`, the same count as the pre-change baseline, and no warning names a touched file. `rustfmt --edition 2021 --check` is clean on all five touched Rust files, and `cargo fmt --all` was never run. No pre-existing rustfmt-dirty file was touched. Every touched file stays under the 1000 physical line limit: `src/cli/intent.rs` 224, `src/core/resolve.rs` 82, `src/planner/apply.rs` 339, `src/planner/mod.rs` 53, `tests/planner_plan_verdict_truth_contract.rs` 312. |
| The worker | the owning agent wrote the change | The worker subagents in this programme have been unreliable — change 34's worker stopped reporting after writing code, change 35's worker wrote everything then stopped, and change 36's worker read the package and ended without writing anything — so the owning agent wrote this change itself. |
