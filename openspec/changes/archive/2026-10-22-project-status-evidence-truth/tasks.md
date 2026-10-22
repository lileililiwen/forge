# Tasks: project-status-evidence-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the status projection: `src/api/status.rs` (`doctor_check`, `check_check`, `readiness_check`, `overall_state`).
- [x] 1.3 Read the doctor report model: `src/doctor/model.rs` (`DoctorReport::healthy`, `blocking_findings`, `unmet_controls`) and the `Finding` statuses.
- [x] 1.4 Read the checker assembly: `src/checker/mod.rs` (the `Unavailable` → warning mapping).
- [x] 1.5 Read the canonical capability: `openspec/specs/forge-web-project-status/spec.md` (the "Honest sub-check states and safe reasons" requirement).
- [x] 1.6 Read the consumers of the route: the fleet summary in `src/api/status.rs`, the workbench health in `src/api/workbench.rs`, and the portal's project tile.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the doctor sub-check judges with the doctor's own blocking set — `FAIL` or `UNAVAILABLE` — so its state partition is the inverse of the doctor's `healthy` field.
- [x] 2.2 Decide the summary: the failing findings, the unavailable findings and the unmet controls are reported apart, so a check that could not run is never counted as a failing check.
- [x] 2.3 Decide the stale case: a stale report with an unavailable finding reports `issues` and names the staleness, because the canonical requirement already demands `issues` for failing or warning evidence.
- [x] 2.4 Decide the guard: the projection's partition and its summary are pinned by a contract check.

## 3. Implement

- [x] 3.1 `src/api/status.rs`: the doctor sub-check judges with `blocking_findings()`, separates the failing count from the unavailable count, and names the staleness when the report carries it.
- [x] 3.2 `src/api/status.rs`: the unit tests for the rule.
- [x] 3.3 `src/api/status.rs`: the `check` and `readiness` sub-checks stay as they are; the fix makes the doctor sub-check agree with the check sub-check.
- [x] 3.4 `tests/project_status_evidence_truth_contract.rs` (new): the pinned partition, summary and agreement.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 The status route for a project whose only problems are unavailable checks: the doctor sub-check names the unavailable count, and the `check` sub-check names the same findings.
- [x] 4.4 The status route for a healthy project: the `healthy` summary is unchanged.
- [x] 4.5 The fleet summary: it counts the states the projection produces.
- [x] 4.6 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.7 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.8 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 1800` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [x] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [x] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capability's purpose.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed.

| Check | Result | Evidence |
|---|---|---|
| 4.1 `cargo build --bin forge` | PASS | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 14.60s`; exit 0. `cargo build --workspace` also finished with `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 16.76s`. |
| 4.2 the repository test suite | PASS | `bash scripts/check-tests.sh` — the Gate's own `tests` command, and the same suite CI's `test` job runs — exited 0 on the final tree. The 222 `test result:` lines sum to **4136 passed; 0 failed; 1 ignored**; `grep -c FAILED` over the whole log returns 0. The pre-change baseline the previous change recorded, measured with the same script, was **4123 passed over 221 lines**, so this change adds exactly the 13 tests it declares (8 unit cases in `src/api/status.rs` + 5 contract cases) and exactly one test binary (the new contract test). The 222 lines break into 217 from the root workspace and 5 from the identity-providers workspace that the script also runs (13 tests). An earlier run of `cargo test --workspace --all-targets` alone, without the identity-providers workspace, reported 4123 over 218 lines; the scope difference — 5 binaries and 13 tests — accounts for the whole difference between the two figures, and neither is wrong. The one ignored test is `tampering::packing_leaves_the_sibling_checkout_byte_identical`. The known flake `checker_contract::format_json_does_not_alter_checker_stdout` passed in this run, so no re-run was needed. |
| 3.4 `cargo test --test project_status_evidence_truth_contract` | PASS | After the fix: `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.76s`. Before the fix the same file failed: `test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.82s`, with `the summary "0 failing finding(s), 0 unmet control(s)" must name the unavailable finding(s) count` (×2) and `assertion \`left == right\` failed: a stale observation never hides a finding the doctor could not gather / left: String("stale") / right: "issues"`. |
| 3.2 `cargo test -p forge --lib api::status` | PASS | `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 1411 filtered out; finished in 0.00s`. The eight cases: the unavailable finding, the failing finding, both apart, the unmet control, the healthy wording, the stale wording, the stale report with an unavailable finding, and the agreement with the check sub-check. |
| 4.3 the status route with unavailable checks | PASS | Driven over HTTP: `forge --registry <db> api serve --bind 127.0.0.1 --port 47311`, then `curl -b cookies.txt http://127.0.0.1:47311/v1/admin/projects/unavailable-project/status -H 'Origin: http://127.0.0.1:4173'`. The fixture is a rust-web L1 project with `forge.yaml`, `Cargo.toml` and `README.md` that is **not** a git repository. **Before**: `{"id":"doctor","state":"issues","summary":"0 failing finding(s), 0 unmet control(s)"}` while `{"id":"check","state":"issues","summary":"0 error(s), 1 warning(s)"}` named the same finding. **After**: `{"id":"doctor","state":"issues","summary":"0 failing finding(s), 1 unavailable finding(s), 0 unmet control(s)"}` and the unchanged `{"id":"check","state":"issues","summary":"0 error(s), 1 warning(s)"}` — both name the one unavailable finding. The stale variant (the manifest rewritten after registration) **before**: `{"id":"doctor","state":"stale","summary":"the registry observation predates the current manifest; re-register to refresh"}`; **after**: `{"id":"doctor","state":"issues","summary":"0 failing finding(s), 1 unavailable finding(s), 0 unmet control(s); the registry observation is stale"}`. |
| 4.4 the status route for a healthy project | PASS | `GET /v1/admin/projects/healthy-project/status` (a rust-web L1 git repository with an `origin` remote) returns `{"id":"doctor","state":"healthy","summary":"14 finding(s), 0 unmet control(s); no failing evidence"}` — byte-identical before and after the change. |
| 4.5 the fleet summary | PASS | `GET /v1/admin/status` returns `{"counts":{"healthy":1,"issues":2,"stale":0,"unavailable":0},"total":3}` with the three registered projects listed as `healthy-project: healthy`, `stale-project: issues`, `unavailable-project: issues`. The counts sum to the total, and the stale project is counted `issues` because its doctor sub-check is `issues`. |
| 4.6 `rustfmt --edition 2021 --check` on every touched Rust file | PASS | `rustfmt --edition 2021 --check src/api/status.rs` and `rustfmt --edition 2021 --check tests/project_status_evidence_truth_contract.rs` both print nothing (exit 0). `cargo fmt --all` was never run. `git status --porcelain` shows only ` M HANDOFF.md` (the owner's change, not mine), ` M src/api/status.rs`, `?? openspec/changes/project-status-evidence-truth/` and `?? tests/project_status_evidence_truth_contract.rs`; none of the twelve pre-existing rustfmt-dirty files (`crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `src/cli/use_cmd.rs`, `tests/delivery_cross_surface.rs`, `tests/forge_web_project_delivery_contract.rs`) is modified. |
| 4.7 `cargo clippy --workspace --all-targets --all-features` | PASS | Exit 0 with **39** warnings — the pre-existing baseline, unchanged. `grep "api/status.rs\|project_status_evidence_truth_contract.rs"` over the clippy log returns nothing: no warning in either touched file. |
| 4.8 the source file line counts of every touched file | PASS | `wc -l src/api/status.rs` → 641; `wc -l tests/project_status_evidence_truth_contract.rs` → 427. Both are under 1000 physical lines. |
| 5.1 `forge gate --dry-run` | PASS | Exit 0: `checks (8 planned)` — build, declared-verification, placeholder-threshold, product-code-boundary, repository, security, source-file-size, tests — with `unbound required checks: none` and `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. Nothing was executed, persisted or journaled. |
| 5.2 `forge gate --timeout-secs 1800` | PASS | Exit 0: `project: forge`, `freshness: fresh`, `aggregate: passed`, `profile: full`, `revision: 3a849825b8d1`, `observed_at: 2026-10-22T14:44:14Z`, `rule_pack_version: forge-baseline/1.0.0`. All eight checks report `pass`, including `source-file-size pass: evaluated 458 of 458 candidate file(s); maximum 1000 physical line(s)` and `build pass: diagnostic: Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 0.11s` — the build was warm before the run. Evidence persisted to `.forge/gate/forge/evidence.json` (gitignored; `git status` stays clean). The revision is the parent commit, because the implementation is still uncommitted when the Gate runs. |
| 5.3 `openspec validate --all --strict --no-interactive` | PASS | Exit 0: `Totals: 145 passed, 0 failed (145 items)`. |
| 5.4 `node scripts/check-openspec-change-names.mjs` | PASS | Exit 0: `check-openspec-change-names: PASS`. |
| 5.5 `git diff --check` | PASS | No output (no whitespace error). |

### Judgement calls

- **The contract test drives the real route.** `crate::api::status` is a
  private module, so the projection is not reachable from `tests/`. Instead
  of adding a public seam, the contract test drives the session-gated route
  through the public `forge::api::handle` and compares it with the report
  the public `forge::doctor::run_doctor` produces for the same directory. No
  public API was widened. The rule's own cases are pinned by the unit tests
  in `src/api/status.rs`.
- **The fixture has exactly one unavailable finding.** The design's example
  names two (the repository inspector and the DriftWatch adapter). The route
  passes no policy outcome, so the doctor emits no DriftWatch policy finding
  at all; the fixture therefore carries one unavailable finding
  (`repository`, because the project is not a git repository). The counts in
  the summary follow the report, whatever it carries.
- **`blocking_findings()` counts every blocking finding, applicable or not.**
  The doctor computes its `healthy` field over all findings, so the
  projection's partition stays the exact inverse of that field. The checker
  counts only applicable findings, so the agreement claim holds in the
  direction the canonical requirement demands: a finding the checker counts
  is a finding the projection counts.
