# Tasks: workbench-health-state-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the workbench health builder: `src/api/workbench.rs` (`build_health`, `refresh_health`, `finish_health`, `unavailable_health`).
- [x] 1.3 Read the doctor report model: `src/doctor/model.rs` (`DoctorReport`, `blocking_findings`, `unmet_controls`) and the runner's `healthy` field in `src/doctor/runner.rs`.
- [x] 1.4 Read the sibling projection: `src/api/status.rs` (`doctor_check`, `overall_state`) — the partition it already applies.
- [x] 1.5 Read the canonical capability: `openspec/specs/workbench-health-latency/spec.md` ("Fast workbench open", "Explicit full health check").
- [x] 1.6 Read the portal's health card: `frontend/app.js` (`HEALTH_LABELS`, `HEALTH_BADGE`, `renderHealth`) and the lifecycle series rail's use of the health state.
- [x] 1.7 Reproduce the defect: a registered project whose manifest is written after the registration reports `issues` on the workbench detail while the sibling status route reports `stale` for the same report.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the doctor's judgement of its own report becomes one typed partition — `Issues`, `Stale`, `Healthy` — judged by the doctor's blocking set, and both projections read it.
- [x] 2.2 Decide the runner: the `healthy` field is expressed through the partition, so the field and the partition cannot drift.
- [x] 2.3 Decide the workbench mapping: `Issues` → `issues`, `Stale` → `stale`, `Healthy` → `healthy` when the live policy pass ran and `deferred` when it did not. `unavailable` stays the workbench's own state for an unreadable directory.
- [x] 2.4 Decide the status-route mapping: unchanged behaviour, read through the partition.
- [x] 2.5 Decide the guard: the two projections are pinned to one partition by a contract check that drives the real routes.

## 3. Implement

- [x] 3.1 `src/doctor/model.rs`: the `HealthPartition` type and the doctor's report partition.
- [x] 3.2 `src/doctor/runner.rs`: the `healthy` field is expressed through the partition.
- [x] 3.3 `src/api/workbench.rs`: the health document's state is derived from the partition.
- [x] 3.4 `src/api/status.rs`: the doctor sub-check reads the partition; its states and wording do not change.
- [x] 3.5 `src/doctor/model.rs`: the unit tests for the partition.
- [x] 3.6 `tests/workbench_health_state_truth_contract.rs` (new): the two projections pinned to one partition, driven through the real routes.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 The workbench detail for a stale project: `stale`, and the document still names the stale observation.
- [x] 4.4 The explicit refresh for a stale project: `stale`.
- [x] 4.5 The workbench detail for a project with a failing local check: `issues`, unchanged.
- [x] 4.6 The workbench detail for a locally-clean project: `deferred` with the `policy-deferred` finding, unchanged.
- [x] 4.7 The explicit refresh for a locally-clean project: `healthy`, unchanged.
- [x] 4.8 The workbench detail for an unreadable project directory: `unavailable`, unchanged.
- [x] 4.9 The project-status doctor sub-check for the same projects: its states and wording are unchanged.
- [x] 4.10 The doctor's own `healthy` field and findings are unchanged for every report.
- [x] 4.11 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.12 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.13 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 1800` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [x] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [x] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capabilities' purposes.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed.

| Check | Result | Evidence |
|---|---|---|
| 1.7 the reproduced defect | PASS | Reproduced before the fix by running the new contract test against the unmodified source: `cargo test --test workbench_health_state_truth_contract` → `test result: FAILED. 3 passed; 1 failed` and the stale case asserted `left: "issues"` / `right: "stale"`. The captured document was `{"healthy":false,"stale":true,"state":"issues",…}` with every finding `pass` except `registry-observation` `warn` and the fast path's own `policy-deferred` `unavailable` row — the state said *issues* while the document named no failing finding. The sibling route already answered `stale`. |
| 4.2 the repository test suite | PASS | `bash scripts/check-tests.sh` → exit 0. `grep -c "test result:"` = **224** lines (baseline 223). Totals: **4159 passed, 0 failed, 1 ignored** (baseline 4148 passed, 0 failed, 1 ignored over 223 lines; +11 tests = 4 contract + 7 partition unit tests). Neither known flake fired in this run, so no re-run was needed. |
| 3.6 the contract test | PASS | `cargo test --test workbench_health_state_truth_contract` → `running 4 tests` … `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.06s`. |
| 4.3 the stale project, detail | PASS | `health.state` = `stale`; `stale` = `true`; `healthy` = `false`; `registry-observation` = `warn`; the `policy-deferred` row is still present; the findings carry **0** blocking rows besides that appended row. |
| 4.4 the stale project, refresh | PASS | `POST /v1/admin/projects/stale-truth/health/refresh` → `health.state` = `stale`. |
| 4.5 a failing local check | PASS | Detail `health.state` = `issues`, refresh `health.state` = `issues`, and the named failing finding is `build-config` with `status: "fail"`. |
| 4.6 a locally-clean project, detail | PASS | `health.state` = `deferred`; `policy-deferred` present with `status: "unavailable"`, `applicable: false`. |
| 4.7 a locally-clean project, refresh | PASS | `health.state` = `healthy`; no `policy-deferred` row in the full pass. |
| 4.8 an unreadable directory | PASS | The registered directory was removed after registration; the detail reports `health.state` = `unavailable` with its explanatory `note`. |
| 4.9 the project-status doctor sub-check | PASS | stale → `state: "stale"`, `summary: "the registry observation predates the current manifest; re-register to refresh"`; failing check → `state: "issues"`; clean project → `state: "healthy"`. The wording and the counts are unchanged — the route now reads the partition instead of re-deriving it. |
| 4.10 the doctor's unchanged report | PASS | `cargo test -p forge --lib doctor::` → `74 passed; 0 failed; 0 ignored`. The runner's `healthy` field is now `matches!(report.health_partition(), HealthPartition::Healthy)`, the same facts it used before (`no FAIL`/`UNAVAILABLE` finding, no unmet control, not stale); the new unit test `the_healthy_partition_is_the_doctors_own_healthy_field` pins the old expression against the partition over ten report shapes. The findings, their statuses and their evidence text are untouched. |
| 4.11 rustfmt | PASS | `rustfmt --edition 2021 --check src/doctor/model.rs src/doctor/runner.rs src/api/workbench.rs src/api/status.rs tests/workbench_health_state_truth_contract.rs` → exit 0, no output. `cargo fmt --all` was never run, and none of the twelve pre-existing rustfmt-dirty files was touched. |
| 4.12 clippy | PASS | `cargo clippy --workspace --all-targets --all-features` → **39 warnings**, the same count as the baseline, and no warning in a touched file (warning locations: `src/registry/model.rs`, `src/hosted_ci/store.rs`, `src/deploy/model.rs`, `src/contract/mod.rs`, `src/cli/auth.rs`, `src/auth/setup.rs`, `tests/origin_repository_retirement_contract.rs`, `tests/deploy_ssh_adapter_contract.rs`, `crates/cargo-sonar/**`, `crates/cargo-audit-boost/**` — all pre-existing). |
| 4.13 the source file line counts | PASS | `src/doctor/model.rs` 334, `src/doctor/runner.rs` 691, `src/api/workbench.rs` 828, `src/api/status.rs` 643, `tests/workbench_health_state_truth_contract.rs` 433 — every touched file is under 1000 physical lines. The Gate's own `source-file-size` check passed: `evaluated 458 of 458 candidate file(s); maximum 1000 physical line(s)`. |
| 5.1 `forge gate --dry-run` | PASS | `./target/debug/forge gate --dry-run` → exit 0. `checks (8 planned)`: build, declared-verification, placeholder-threshold, product-code-boundary, repository, security, source-file-size, tests — every one `[required]`. `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. |
| 5.2 `forge gate --timeout-secs 1800` | PASS | The owning agent re-ran the Gate on the final tree: **exit 0**, `project: forge`, `freshness: fresh`, `aggregate: passed`, `profile: full`, `revision: c1bbe57320e4`, `observed_at: 2026-10-23T01:24:02Z`, `manifest_digest: sha256:2c456f0e82a570fb`, `rule_pack_version: forge-baseline/1.0.0`. All eight checks report `pass` — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` — and the `build` check reported `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 0.12s` against a warmed tree. The worker's earlier Gate run on the same tree also passed all eight checks, at `observed_at: 2026-10-23T00:58:38Z`; the `security` check reached the advisory database in both runs, so there was no network fault. The revision is the parent commit, because the implementation is still uncommitted in the worktree when the Gate runs; the Gate's own `repository` check ran against that worktree. Evidence persisted to `.forge/gate/forge/evidence.json` (git-ignored). |
| 5.3 strict validation | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 147 passed, 0 failed (147 items)`, including `✓ change/workbench-health-state-truth` and `✓ spec/workbench-health-latency`. The count is unchanged from the pre-change baseline. |
| 5.4 the change names | PASS | `node scripts/check-openspec-change-names.mjs` → `check-openspec-change-names: PASS` (exit 0). |
| 5.5 `git diff --check` | PASS | `git diff --check` → exit 0, no output. |

### Judgement calls

- **The partition lives in the doctor.** The doctor judges its own
  report, and both projections are already consumers of it. One typed
  partition is one rule in one place.
- **The doctor's `healthy` field does not change.** The runner expresses
  it through the partition, and the partition is the same facts the
  runner already used, so every report keeps the same field value.
- **The project-status route does not change.** It already reported
  `stale`; it now reads the partition instead of re-deriving it.
- **`deferred` and `unavailable` stay the workbench's own states.** The
  first says the live policy check did not run for this view, the second
  that the project directory could not be read. Neither is a partition
  of the doctor's report.
- **The contract test drives the real routes.** It calls the public
  `forge::api::handle` for the workbench detail, the explicit refresh
  and the project-status route, so no public seam is widened and no
  private module is opened.
- **The contract test needs a live policy checker for the refresh
  route.** DriftWatch is not installed on this host, so an unconfigured
  refresh would produce an `unavailable` `driftwatch-policy` finding and
  every report would read as `issues`. The test therefore points
  `FORGE_DRIFTWATCH_BIN` at a fixture script that reports one **passing**
  policy finding, so the refresh's own verdict is `passed` and the only
  blocking evidence in the fixtures is the fixture's own. The env
  override is process-scoped, so the tests in the file serialize on a
  mutex, as `tests/workbench_health_latency_contract.rs` does.
- **The stale fixture is a fully clean project.** The stale case only
  reaches `stale` when the report carries no blocking evidence, so the
  fixture is the `workbench-health-latency` clean project (git remote,
  `Cargo.toml`, `README.md`, `Dockerfile`, CI workflow, `driftwatch.yaml`)
  with `forge.yaml` written again after the registration.
- **The before-state was captured, not reasoned.** The new contract test
  was written first and run against the unmodified source; the stale
  case failed with `left: "issues"` / `right: "stale"`. The tree was
  never stashed.
