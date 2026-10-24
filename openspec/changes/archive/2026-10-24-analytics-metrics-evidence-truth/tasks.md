# Tasks: analytics-metrics-evidence-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the aggregate: `src/analytics/projections.rs` (`aggregate_project_metrics`, `count_metric`, the repository-star aggregate).
- [x] 1.3 Read the doctor summary: `src/analytics/model.rs` (`DoctorSummary`).
- [x] 1.4 Read the metrics CLI: `src/cli/functions_13.rs` (`cmd_analytics_metrics`, the journal row).
- [x] 1.5 Read the assurance browser's metrics route: `src/api/admin/assurance/analytics.rs`.
- [x] 1.6 Read the canonical capability: `openspec/specs/external-planes-analytics/spec.md` ("Timestamped project metrics").
- [x] 1.7 Read the existing tests: `tests/analytics_contract.rs`, `tests/analytics_cross_surface.rs`, `tests/web_assurance_browser_contract.rs`, `src/analytics/projections_tests.rs`.
- [x] 1.8 Reproduce the defect on the current tree and record the fabricated metric, its snapshot and the report's completion flag.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: a metric states a value only when Forge observed it; an unmeasured metric is `unavailable` with a note naming the missing evidence.
- [x] 2.2 Decide where the rule lives: in the aggregator, next to the aggregate it builds.
- [x] 2.3 Decide which metrics the rule governs: the eight doctor-sourced metrics, and only those — the registry count and the repository-star metrics keep their own evidence.
- [x] 2.4 Decide that `complete` is true only when every metric was measured.
- [x] 2.5 Decide that the fabricated summary cannot be built again.
- [x] 2.6 Decide the untouched surfaces: the repository-star metrics, the provider observation path, the summary file's shape, the exit code, the contract version, the inspection lane.

## 3. Implement

- [x] 3.1 `src/analytics/projections.rs`: the doctor metrics follow the evidence or are `unavailable`; the `unavailable_metric` helper.
- [x] 3.2 `src/analytics/model.rs`: `DoctorSummary` loses its `Default` derive, with the reason in its doc.
- [x] 3.3 `src/cli/functions_13.rs`: no fabricated summary; the journal detail names the unavailable count.
- [x] 3.4 `src/api/admin/assurance/analytics.rs`: no fabricated summary.
- [x] 3.5 `src/analytics/projections_tests.rs`: the rule's unit coverage, both ways.
- [x] 3.6 `tests/analytics_metrics_evidence_truth_contract.rs`: the CLI and the assurance browser contract for an unmeasured metric.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [ ] 4.2 The repository test suite passes.
- [x] 4.3 The doctor-sourced metrics are `unavailable`, carry no snapshot and no current value, and name the missing evidence.
- [x] 4.4 No doctor-sourced metric carries a source and an observation timestamp.
- [x] 4.5 The registry-count metric is still `present` with the real count.
- [x] 4.6 The report is not complete while a metric is unmeasured, and the journal row is `partial` with the unavailable count in its detail.
- [x] 4.7 A gathered doctor summary still produces the real counts, and the report is complete when the repository metrics are available too.
- [x] 4.8 The assurance browser's metrics route reports the same gap and still probes nothing and writes nothing.
- [x] 4.9 The repository-star metrics, the provider observation path, the summary file's shape and the exit code are unchanged.
- [x] 4.10 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.11 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.12 Every touched file stays under the 1000 physical line limit.

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
The Gate rows are the owning agent's run: this worker did not run
`forge gate`, so 5.1, 5.2 and section 6 stay unchecked.

| Check | Command | Observed |
|---|---|---|
| Build | `cargo build --workspace` | Exit 0 — `Finished dev profile [unoptimized + debuginfo] target(s)`. |
| Analytics unit tests | `cargo test -p forge --lib analytics::` | `test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 1430 filtered out`. Includes `metrics_without_a_doctor_summary_report_unavailable_metrics` and `metrics_aggregate_counters_present_with_windows`. |
| New contract test | `cargo test --test analytics_metrics_evidence_truth_contract` | `test result: ok. 2 passed; 0 failed` — `a_metrics_run_reports_the_unmeasured_doctor_metrics_as_unavailable` and `the_assurance_browser_reports_the_same_gap`. |
| Analytics contracts | `cargo test --test analytics_contract --test analytics_cross_surface` | `analytics_contract`: `test result: ok. 17 passed; 0 failed`. `analytics_cross_surface`: `test result: ok. 6 passed; 0 failed`. No assertion in either file contradicts the new rule; only the R2 success wording in the `analytics_contract` header changed. |
| Assurance browser contract | `cargo test --test web_assurance_browser_contract` | `test result: ok. 12 passed; 0 failed`. `analytics_metrics_probe_nothing_and_write_nothing` still passes: the route journals nothing and writes no summary file. |
| Full suite | `bash scripts/check-tests.sh` | The owning agent's run: exit 0; the `233` `test result:` lines sum to **4223 passed; 0 failed; 1 ignored**. The pre-change baseline, measured the same way, was 232 lines and 4220 passed, so this change adds exactly the 3 tests it declares (1 unit case and 2 contract cases) and one new test binary. |
| Repository Gate | `forge gate --timeout-secs 2400` | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-24T07:54:39Z"` on revision `5cf841a0c8a1c45618afe0f7fc8f5a07e655451c` (the parent commit — the implementation is uncommitted while the Gate runs), with all eight required checks — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` — in state `pass`. The `source-file-size` check evaluated 459 of 459 candidate files under the 1,000-line limit. The rehearsal before it (`forge gate --dry-run`) resolved the same 8 required checks, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. The advisory database's fetch was resolved to the local mirror of the same cached database (`/home/paul/.cache/advisory-db-mirror.git`, commit `640bf5a9`) through `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_0=url.file:///home/paul/.cache/advisory-db-mirror.git.insteadOf` / `GIT_CONFIG_VALUE_0=https://github.com/RustSec/advisory-db.git`, and `CARGO_HTTP_PROXY=http://127.0.0.1:1` made the unreachable crates.io index update fail fast instead of waiting for the TCP timeout. Both settings applied to that one Gate process only: no file in this repository and no global cargo or git configuration was changed. |
| Lint | `cargo clippy --workspace --all-targets --all-features` | The owning agent's run: exit 0 with `39` lines starting with `warning`, the same count as the pre-change baseline, and no warning names a touched file. The remaining warnings are pre-existing in untouched files: `src/auth/setup.rs`, `src/contract/mod.rs`, `src/deploy/model.rs`, `src/hosted_ci/store.rs`, `src/registry/model.rs`, `src/cli/auth.rs`, `crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `tests/deploy_ssh_adapter_contract.rs`, `tests/origin_repository_retirement_contract.rs`. |
| Format | `rustfmt --edition 2021 --check <file>` on each touched file | All seven touched files clean. Only the two files this change made dirty were formatted (`src/analytics/projections_tests.rs`, `tests/analytics_metrics_evidence_truth_contract.rs`); no rustfmt-dirty file outside this change was touched. |
| File size | `wc -l` on each touched file | `src/analytics/projections.rs` 915, `src/analytics/model.rs` 364, `src/cli/functions_13.rs` 988, `src/api/admin/assurance/analytics.rs` 76, `src/analytics/projections_tests.rs` 716, `tests/analytics_contract.rs` 582, `tests/analytics_metrics_evidence_truth_contract.rs` 359 — every file under 1000 physical lines. |
| Whitespace | `git diff --check` | Exit 0, no finding. |
| Spec validation | `node .../openspec.js validate --all --strict --no-interactive` | `Totals: 157 passed, 0 failed (157 items)`. |
| Change names | `node scripts/check-openspec-change-names.mjs` | `check-openspec-change-names: PASS`. |
| Defect before the change | `forge --registry <db> --format json analytics metrics <project>` | `quality-healthy` reported `"state": "present"`, `"current": 0` and one snapshot `{"source": "doctor", "value": 0, "observed_at": "2026-10-24T07:07:51.085360065+00:00", "window": "instant"}`; the report's `complete` was `true`. The doctor never ran. |
| Defect after the change | same command | `quality-healthy` reports `"state": "unavailable"`, `"current": null`, `"snapshots": []`, `"windows": []` and the note `no doctor summary was gathered for this run; this metric was not measured`; the same holds for `quality-warnings`, `quality-failures`, `agents-running`, `specs-queued`, `deployment-running`, `deployment-offline` and `deployment-pending`; `projects` stays `present` with the real count; the report's `complete` is `false`. |
| Journal row after the change | `forge analytics metrics <project>` then read the `operations` row | `kind=analytics`, `state=partial`, `detail=metrics: scope=ana-truth default_window=7d complete=false aggregates=11 unavailable=8`. The eight unavailable metrics are the eight doctor-sourced ones; the fixture's repository provider answered, so the repository metrics are measured. |

### The owning agent's additions

- The worker wrote the five source files, the unit tests and the contract
  test, then stopped reporting. The owning agent re-ran every check on
  the final tree — build, the analytics unit tests, the analytics contract
  tests, the assurance browser contract, the full repository suite, clippy,
  rustfmt, the file-size limit, the strict spec validation, the
  change-name preflight and the Gate — and simplified the constant that
  lists the doctor-sourced metrics: it carried a source column the
  gathered branch names in full, so the column was dead weight that could
  drift. The constant now lists the metric ids, and the design document
  was updated to match. The Gate run is the owning agent's, because the
  worker does not own the Gate.
