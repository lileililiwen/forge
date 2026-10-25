# Tasks: fleet-publish-summary-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the fleet lane: `src/cli/fleet.rs` (the three publish lanes, the summary, `fleet_project_verdict`).
- [x] 1.3 Read the publish rule: `src/publish/model.rs` (`PublishVerdict`, `verdict_for_report`).
- [x] 1.4 Read the canonical capability: `openspec/specs/publish-fleet-dry-run-verdict-truth/spec.md`.
- [x] 1.5 Read the existing tests: `tests/publish_fleet_dry_run_verdict_truth_contract.rs`, `tests/fleet_live_rollout_contract.rs`.
- [x] 1.6 Reproduce the defect on the current tree and record the summary it printed.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the fleet summary counts the verdict — a delivery is a success, a plan is a plan, a failure is a failure.
- [x] 2.2 Decide where the rule lives: the fleet lane, next to the verdict rule it already reads.
- [x] 2.3 Decide the options and reject the ones that keep the binary counting or that hide the plans.
- [x] 2.4 Decide the untouched surfaces: the per-project rows, the journal rows, the fail-fast behaviour, the exit status, the upgrade fleet lane.

## 3. Implement

- [x] 3.1 `src/cli/fleet.rs`: the counters come from the verdict, in all three fleet lanes.
- [x] 3.2 `src/cli/fleet.rs`: the summary document gains the `planned` counter and the human line names the planned runs.
- [x] 3.3 `tests/fleet_publish_summary_verdict_truth_contract.rs`: the plan, the delivery and the failure.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A planned publish is counted as a plan, never as a success.
- [x] 4.4 A delivered publish is counted as a success in every lane.
- [x] 4.5 A failed publish is counted as a failure; the fail-fast behaviour and the exit status are unchanged.
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

- [x] 6.1 Archive without `--skip-specs`, so the modified capability is promoted.
- [x] 6.2 Update the modified capability's purpose and strip the archive's trailing blank lines by hand.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

### The reproduced defect

```
$ forge --registry /tmp/fleet/registry.db publish fleet \
    --inventory /tmp/fleet/inventory.json --provider probe --dry-run
publish alpha: provider=probe
fleet publish: 1/1 publishable succeeded, 0 held back, 0 skipped (1 in inventory:...)
```

The run was a dry run: it contacted nobody and deployed nothing. The
lane's own per-project row said `verdict: planned`, `healthy: false`,
and the journal row said `planned`. The summary said "succeeded".

The JSON document said the same in numbers:

```json
"fleet": {"publishable": 1, "success": 1, "failed": 0,
          "projects": [{"project": "alpha", "healthy": false, "verdict": "planned"}]}
```

The counters were a binary, not the verdict, and the code's own comment
said what the code did not do:

```rust
// src/cli/fleet.rs, before
// A plan is not a failure: the fleet counts it
// as neither, and it never trips `--fail-fast`.
if verdict == PublishVerdict::Failed {
    failure_count += 1;
    ...
} else {
    success_count += 1;
}
```

The same binary sat in the other two fleet lanes, and the parallel lane
had the other half of the same bug: no `else` branch at all, so a
delivery there was counted as nothing.

### The change

| Surface | Before | After |
|---|---|---|
| `src/cli/fleet.rs` counters | a binary (`success_count` / `failure_count`) | `FleetCounters`, counted from the verdict |
| `src/cli/fleet.rs` parallel lane | a delivery was counted as nothing | a delivery is a success |
| `src/cli/fleet.rs` provider lane | a plan was counted as a success | a plan is a plan |
| `src/cli/fleet.rs` adapter lane | a plan was counted as a success | a plan is a plan |
| `src/cli/fleet.rs` summary document | `success`, `failed` | `success`, `planned`, `failed` |
| `src/cli/fleet.rs` human line | "N/M publishable succeeded, …" | "N/M publishable succeeded, P planned, …" |

### The checks

| Check | Result | Evidence |
|---|---|---|
| The reproduced defect | reproduced before the change | The line above: "1/1 publishable succeeded" for a dry run whose own row said `verdict: planned`. |
| The contract test fails without the change | PASS | With the counter put back to the binary (`Delivered | Planned => self.success += 1`), `a_planned_fleet_publish_is_not_counted_as_a_success` fails with `left: 1, right: 0` on the summary's `success` counter, while the summary's own project row says `verdict: planned`. The other two cases pass on both trees, as they should. |
| A plan is never counted as a success | PASS | `a_planned_fleet_publish_is_not_counted_as_a_success`: the summary counts `success: 0`, `planned: 1`, `failed: 0`, and the project row agrees with `verdict: planned`. |
| The summary names the planned runs | PASS | The same case reads the human line: "0/1 publishable succeeded" and "1 planned". |
| A delivery is counted as a success | PASS | `a_delivered_fleet_publish_is_counted_as_a_success`: the summary counts `success: 1`, `planned: 0`, `failed: 0`, and the human line reads "1/1 publishable succeeded". |
| A failure is counted as a failure | PASS | `a_failed_fleet_publish_is_counted_as_a_failure`: the summary counts `success: 0`, `failed: 1`, and the command still fails. |
| The untouched surfaces | PASS | `git diff` touches no line outside `src/cli/fleet.rs`. The per-project rows, the journal rows, the fail-fast behaviour and the exit status are unchanged: the existing suites pass unchanged — `publish_fleet_dry_run_verdict_truth_contract` 3 passed, `fleet_live_rollout_contract` 9 passed, `publish_observability_contract` 8 passed. The upgrade fleet lane already counted each status separately and is untouched. |
| Repository test suite | PASS | `bash scripts/check-tests.sh` — the repository Gate's own `tests` command, and the same suite CI's `test` job runs — exited 0; the 248 `test result:` lines sum to **4293 passed; 0 failed; 1 ignored**. The pre-change baseline was 247 lines and 4290 passed, so this change adds exactly the 3 tests it declares and one new test binary. |
| Repository Gate | PASS, 8/8 | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-25T12:45:41Z"` on revision `0b8b275c6ac04ff5caffd2fb63f70817a9c389dd` (the parent commit — the implementation is uncommitted while the Gate runs), with all eight required checks — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` — in state `pass`. The rehearsal before it (`forge gate --dry-run`) resolved the same 8 required checks, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. The advisory database's fetch was resolved to the local mirror of the same cached database (`/home/paul/.cache/advisory-db-mirror.git`, commit `640bf5a9`) through `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_0=url.file:///home/paul/.cache/advisory-db-mirror.git.insteadOf` / `GIT_CONFIG_VALUE_0=https://github.com/RustSec/advisory-db.git`, and `CARGO_HTTP_PROXY=http://127.0.0.1:1` made the unreachable crates.io index update fail fast instead of waiting for the TCP timeout. Both settings applied to that one Gate process only. |
| OpenSpec, change names, whitespace | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 167 passed, 0 failed (167 items)`; `node scripts/check-openspec-change-names.mjs` → `PASS`; `git diff --check` and `git diff --cached --check` → no finding. Archived without `--skip-specs`; the `publish-fleet-dry-run-verdict-truth` requirement was modified with all four canonical scenario names kept and two scenarios added, and the archive's trailing blank lines were stripped from the canonical spec by hand. `openspec list` reports no active changes. |
| Lint and formatting | PASS | `cargo clippy --workspace --all-targets --all-features` exited 0 with 39 lines starting with `warning`, the same count as the pre-change baseline, and no warning names a touched file. Three `collapsible_if` warnings the first cut carried were fixed before the archive. `rustfmt --edition 2021 --check` is clean on both touched Rust files, and `cargo fmt --all` was never run. No pre-existing rustfmt-dirty file was touched. Every touched file stays under the 1000 physical line limit: `src/cli/fleet.rs` 890, `tests/fleet_publish_summary_verdict_truth_contract.rs` 281. |
| The worker | the owning agent wrote the change | The worker subagents in this programme have been unreliable, so the owning agent wrote this change. The `explore` subagent's hunt was still running when the owning agent found this candidate by reading the fleet lane; the owning agent verified it, reproduced it, and wrote the rule, the three lanes and the tests. |
