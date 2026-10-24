# Tasks: deploy-dry-run-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the report: `src/deploy/model.rs` (`DeployReport`), and the publish verdict it mirrors: `src/publish/model.rs` (`PublishVerdict`).
- [x] 1.3 Read the apply: `src/deploy/engine/stages.rs` (`apply_deploy`, the note, the health, `journal_report`).
- [x] 1.4 Read the CLI: `src/cli/publish.rs` (`cmd_deploy_apply`).
- [x] 1.5 Read the API view: `src/api/admin/deploy_views.rs` (`report_view`).
- [x] 1.6 Read the canonical capability: `openspec/specs/adapter-deployment/spec.md` ("Observed health and recovery", "Versioned deploy executor contract").
- [x] 1.7 Read the existing tests: `tests/deploy_contract.rs`, `tests/deploy_cross_surface.rs`, `src/deploy/engine/stages_tests.rs`.
- [x] 1.8 Reproduce the defect on the current tree and record the summary line, the journal row and the absence of a deploy state file.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: a run that deployed nothing planned, a run that deployed and completed every stage delivered, anything else failed.
- [x] 2.2 Decide where the rule lives: in the deploy model, next to the report it judges.
- [x] 2.3 Decide the verdict's names, its wire shape and its journal state.
- [x] 2.4 Decide that `healthy` is derived from the verdict and that the exit code follows the verdict.
- [x] 2.5 Decide the note a rehearsal writes.
- [x] 2.6 Decide the untouched surfaces: the adapter's stage outcomes, the deploy state file, the ship-gate refusal, the adapter invocation, the redaction, the observe lane, the contract version.

## 3. Implement

- [x] 3.1 `src/deploy/model.rs`: `DeployVerdict`, `verdict_for_report`, `DeployReport.verdict`.
- [x] 3.2 `src/deploy/engine/stages.rs`: the verdict, the derived health, the note, and the journal detail.
- [x] 3.3 `src/cli/publish.rs`: the journal state follows the verdict; the exit code follows the verdict.
- [x] 3.4 `src/api/admin/deploy_views.rs`: the view carries the verdict.
- [x] 3.5 `src/deploy/engine/stages_tests.rs`: the verdict's unit coverage.
- [x] 3.6 `tests/deploy_dry_run_verdict_truth_contract.rs`: the CLI and the API contract for a rehearsal, a delivery and a failure.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A rehearsal's verdict is `planned`, the report is not healthy, the note says nothing was deployed, the journal records `planned` and the exit code is 0.
- [x] 4.4 A rehearsal writes no deploy state.
- [x] 4.5 A real deploy's verdict is `delivered`, the report is healthy, the journal records `done` and the exit code is 0.
- [x] 4.6 A failed deploy's verdict is `failed`, the journal records `failed` and the exit code is 1 with the report on stdout.
- [x] 4.7 The API's deploy view carries the verdict and the derived health.
- [x] 4.8 The adapter's stage outcomes, the ship-gate refusal, the adapter invocation, the redaction and the observe lane are unchanged.
- [x] 4.9 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.10 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.11 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 2400` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [ ] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [ ] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capability's purpose.
- [ ] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

All commands were run from the repository root with
`source /tmp/forge-env.sh` (so `CARGO_TARGET_DIR=/home/paul/code/forge/target`).

| Check | Command | Observed |
|---|---|---|
| Build | `cargo build --workspace` | Exit 0 — `Finished dev profile [unoptimized + debuginfo] target(s)`. |
| Deploy unit tests | `cargo test -p forge --lib deploy::` | `test result: ok. 52 passed; 0 failed` — 49 pre-existing plus the 3 new cases in `src/deploy/model.rs`: `a_rehearsal_is_a_plan_and_never_a_delivery`, `a_complete_run_is_a_delivery_and_a_broken_run_is_a_failure` and `the_verdict_publishes_a_label_and_a_journal_state`. |
| New contract test | `cargo test --test deploy_dry_run_verdict_truth_contract` | `test result: ok. 3 passed; 0 failed` — `a_rehearsal_is_planned_and_journals_a_planned_row`, `a_completed_deploy_is_delivered_and_journals_a_done_row` and `a_failed_deploy_is_failed_and_journals_a_failed_row`. |
| Deploy contracts | `cargo test --test deploy_contract --test deploy_cross_surface` | `deploy_contract`: `test result: ok. 21 passed; 0 failed`. `deploy_cross_surface`: `test result: ok. 5 passed; 0 failed`. No assertion changed: `dry_run_apply_never_persists_state` still finds the adapter's own `delivered` stage status in a rehearsal, because the adapter's envelope is recorded exactly as the contract requires. |
| Full suite | `bash scripts/check-tests.sh` | The owning agent's run on the final tree: exit 0; the `234` `test result:` lines sum to **4231 passed; 0 failed; 1 ignored**. The pre-change baseline, measured the same way, was 233 lines and 4223 passed, so this change adds exactly the 8 tests it declares (3 unit cases for the rule, 2 unit cases for the API view and 3 contract cases) and one new test binary. |
| Repository Gate | `forge gate --timeout-secs 2400` | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-24T09:06:14Z"` on revision `6aeea72dd50aab25b1d1bcbc636288f9f369888f` (the parent commit — the implementation is uncommitted while the Gate runs), with all eight required checks — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` — in state `pass`. The `source-file-size` check evaluated 459 of 459 candidate files under the 1,000-line limit. The rehearsal before it (`forge gate --dry-run`) resolved the same 8 required checks, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. The advisory database's fetch was resolved to the local mirror of the same cached database (`/home/paul/.cache/advisory-db-mirror.git`, commit `640bf5a9`) through `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_0=url.file:///home/paul/.cache/advisory-db-mirror.git.insteadOf` / `GIT_CONFIG_VALUE_0=https://github.com/RustSec/advisory-db.git`, and `CARGO_HTTP_PROXY=http://127.0.0.1:1` made the unreachable crates.io index update fail fast instead of waiting for the TCP timeout. Both settings applied to that one Gate process only: no file in this repository and no global cargo or git configuration was changed. |
| Lint | `cargo clippy --workspace --all-targets --all-features` | Exit 0 with `39` lines starting with `warning`, the same count as the pre-change baseline, and no warning names a touched file. The `src/deploy/model.rs:245` lifetime warning is pre-existing: it is on `refuse_missing_target_fields`, which this change does not touch. |
| Format | `rustfmt --edition 2021 --check <file>` on each touched file | All six touched Rust files clean. `cargo fmt --all` was never run and no pre-existing rustfmt-dirty file was touched. |
| File size | `wc -l` on each touched file | `src/deploy/model.rs` 872, `src/deploy/engine/stages.rs` 728, `src/deploy/engine/stages_tests.rs` 952, `src/cli/publish.rs` 734, `src/api/admin/deploy_views.rs` 203, `tests/deploy_dry_run_verdict_truth_contract.rs` 339 — every file under the 1000 physical line limit. |
| Whitespace | `git diff --check` | Exit 0, no finding. |
| Spec validation | `node .../openspec.js validate --all --strict --no-interactive` | `Totals: 157 passed, 0 failed (157 items)`. |
| Change names | `node scripts/check-openspec-change-names.mjs` | `check-openspec-change-names: PASS`. |

### The owning agent's additions

- The implementation worker read the package and the code and then ended
  without writing anything, so the owning agent wrote the change itself.
- The API's deploy report view is covered by a unit test in
  `src/api/admin/deploy_views.rs` rather than through the admin route: the
  admin route runs a real deploy and has no rehearsal path, so a planned
  verdict can only be built by the engine and read by the view.
- The three stage-level tests were first written into
  `src/deploy/engine/stages_tests.rs`, which pushed that file past the
  1000 physical line limit. They moved to `src/deploy/model.rs`, next to
  the rule they cover, and the contract test carries the end-to-end
  coverage.

### The reproduced defect

Before the change, on the tree the owning agent measured:

```
$ forge deploy apply /tmp/dep-probe/proj --confirm --dry-run
...
summary: deploy applied to target `home` and observed as `running`
$ python3 -c "import sqlite3; ..."
('deploy', 'dep-probe', 'done', 'deploy `dep-probe-home-b6630d40797e` target=`home` adapter=`local` stages=apply=delivered,observe=running healthy=true dry_run=true')
$ ls /tmp/dep-probe/proj/.forge/deploy
ls: cannot access '/tmp/dep-probe/proj/.forge/deploy': No such file or directory
```

After the change, the same command:

```
summary: dry-run: 2 stage(s) rehearsed; nothing was deployed
('deploy', 'dep-probe', 'planned', 'deploy `dep-probe-home-b6630d40797e` target=`home` adapter=`local` stages=apply=delivered,observe=running verdict=planned healthy=false dry_run=true')
```

The exit code is 0 in both runs: a rehearsal is a successful rehearsal.
