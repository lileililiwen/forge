# Tasks: delivery-provider-health-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the delivery status projection: `src/delivery/projection.rs` (`DeliveryVerbReport`, `compute_phase`, `parse_evidence_and_recovery`, the row→report conversion).
- [x] 1.3 Read the delivery handlers: `src/delivery/handlers.rs` (`encode_provider_detail` — where the provider's health is recorded).
- [x] 1.4 Read the phase vocabulary: `src/delivery/state.rs` (`DeliveryPhase`, `Degraded`).
- [x] 1.5 Read the doctor's use of the phase: `src/doctor/delivery.rs`.
- [x] 1.6 Read the canonical capability: `openspec/specs/project-to-production-workflow/spec.md` ("Read-only delivery status").
- [x] 1.7 Reproduce the defect: a provider that answers `health: "degraded"` with a successful build and run is projected as `phase=healthy`.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the promote branch of the phase computation treats a provider that reported anything other than `healthy` as `degraded`.
- [x] 2.2 Decide the absent case: a row whose detail carries no `provider_health` keeps today's verdict; the projection does not invent one from the absence.
- [x] 2.3 Decide the comparison: exact — the healthy value is the adapter's own `healthy` string.
- [x] 2.4 Decide the other branches: the stage, preflight and Hermora branches are untouched.
- [x] 2.5 Decide the guard: a contract test drives the real workflow against a stubbed provider and holds the phase to the provider's answer.

## 3. Implement

- [x] 3.1 `src/delivery/projection.rs`: the per-verb report carries the provider's health, read from the operation row's detail beside the evidence and recovery lines.
- [x] 3.2 `src/delivery/projection.rs`: the promote branch of the phase computation reads it.
- [x] 3.3 `tests/delivery_provider_health_truth_contract.rs` (new): the two cases, driven through the real CLI with a stubbed provider.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A provider that reports `degraded`: the projection reports `degraded`.
- [x] 4.4 A provider that reports `healthy`: the projection reports `healthy`, unchanged.
- [x] 4.5 A provider that reports no health: the projection keeps its existing verdict.
- [x] 4.6 A promote row whose build or run status is not successful: `degraded`, unchanged.
- [x] 4.7 The phase vocabulary, the rendered names and the delivery contract version are unchanged.
- [x] 4.8 The doctor's `delivery-state` finding is unchanged.
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

- [x] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [x] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capabilities' purposes.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed.

| Check | Result | Evidence |
|---|---|---|
| 1.7 the reproduced defect | reproduced | With `src/delivery/projection.rs` reverted to `HEAD` (`git checkout -- src/delivery/projection.rs`, the rest of the change untouched) and the contract test unchanged, `cargo test --test delivery_provider_health_truth_contract` failed: `test a_provider_that_reports_unhealthy_is_projected_as_degraded ... FAILED`, `left: "healthy"`, `right: "degraded"`. The printed report answered `"phase":"healthy"` for a provider that answered `health: "degraded"` with `build_status: "succeeded"`, `run_status: "succeeded"` and `status: "succeeded"`. This run is this worker's, not only the owning agent's; the owning agent's run showed the same failure. |
| 4.2 the repository test suite | PASS | `bash scripts/check-tests.sh` → 225 `test result:` lines, **4169 passed, 0 failed, 1 ignored**. The pre-change baseline was 224 lines and 4159 passed, 0 failed, 1 ignored; the +10 are the new tests (the contract test binary with 4 tests — 2 written by the owning agent, 2 added here — and 6 projection unit tests). The same command was run twice, once on the tree before an unused-parameter rename in the contract test and once on the final tree; both runs reported 225 lines and 4169 passed, 0 failed, 1 ignored. Neither known flake fired: `checker_contract::format_json_does_not_alter_checker_stdout` and `profile_build_command_runnable_contract` both passed, so no re-run was needed. |
| 3.3 the contract test | PASS | `cargo test --test delivery_provider_health_truth_contract` → `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. The four: `a_provider_that_reports_unhealthy_is_projected_as_degraded`, `a_provider_that_reports_healthy_is_still_projected_as_healthy`, `a_provider_that_reports_no_health_keeps_the_existing_verdict`, `a_promote_whose_build_failed_is_projected_as_degraded`. |
| 4.3 the degraded provider | PASS | `a_provider_that_reports_unhealthy_is_projected_as_degraded ... ok` — the projection answers `phase=degraded` for a provider that answered `health: "degraded"`. The projection unit test `a_promote_row_whose_provider_reported_degraded_is_degraded ... ok` holds the same rule at the row level. |
| 4.4 the healthy provider | PASS | `a_provider_that_reports_healthy_is_still_projected_as_healthy ... ok` — the projection answers `phase=healthy`, unchanged. The projection unit test `a_promote_row_whose_provider_reported_healthy_is_healthy ... ok`. |
| 4.5 a provider with no health | PASS | `a_provider_that_reports_no_health_keeps_the_existing_verdict ... ok` — the projection answers `phase=healthy`, the verdict it gives a successful build and run. The stub answers `"health":""`: the adapter contract makes `health` a required response field (`PublishProviderResponse.health` has no serde default), so an omitted field is not a legal provider response and an empty health is. The row-level boundaries are held by the projection unit tests `a_promote_row_without_a_provider_health_keeps_the_healthy_verdict ... ok` (a detail with no `provider_health` key) and `a_promote_row_whose_detail_is_not_json_keeps_the_healthy_verdict ... ok`. |
| 4.6 a failed build or run | PASS | `a_promote_whose_build_failed_is_projected_as_degraded ... ok` — the promote answers `build_status: "failed"` and the projection answers `phase=degraded`, unchanged. The projection unit test `a_promote_row_whose_build_failed_is_degraded ... ok`. |
| 4.7 the unchanged vocabulary | PASS | `git status --porcelain` lists `src/delivery/projection.rs` as the only changed file under `src/`; `src/delivery/state.rs`, `src/doctor/delivery.rs` and `src/delivery/handlers.rs` are untouched. `DeliveryPhase::Degraded` still renders `degraded` and `DELIVERY_CONTRACT_VERSION` is still `forge-delivery-status/0.1.0` (`src/delivery/mod.rs:33`). `cargo test --test delivery_contract` → `test result: ok. 21 passed; 0 failed; 0 ignored`. |
| 4.8 the doctor's finding | PASS | `cargo test -p forge --lib doctor::` → `test result: ok. 74 passed; 0 failed; 0 ignored; 1369 filtered out`. `src/doctor/delivery.rs` and `src/doctor/observation.rs` are unchanged; the doctor already maps `Degraded` to `FAIL`. |
| 4.9 rustfmt | PASS | `rustfmt --edition 2021 --check src/delivery/projection.rs` and `rustfmt --edition 2021 --check tests/delivery_provider_health_truth_contract.rs` both exit 0 with no diff. No pre-existing rustfmt-dirty file was touched: `crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `src/cli/use_cmd.rs`, `tests/delivery_cross_surface.rs` and `tests/forge_web_project_delivery_contract.rs` are unchanged. |
| 4.10 clippy | PASS | `cargo clippy --workspace --all-targets --all-features` → 39 warnings, the same count as the baseline. No warning points at `src/delivery/projection.rs` or at `tests/delivery_provider_health_truth_contract.rs`. |
| 4.11 the source file line counts | PASS | `src/delivery/projection.rs` 698 physical lines, `tests/delivery_provider_health_truth_contract.rs` 336, `openspec/changes/delivery-provider-health-truth/tasks.md` 107 — every touched file is under 1000. The Gate's own `source-file-size` check reported `evaluated 458 of 458 candidate file(s); maximum 1000 physical line(s)` and passed. |
| 5.1 `forge gate --dry-run` | PASS | Exit 0. `checks (8 planned)`: `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests` — every one `[required]`. `unbound required checks: none`; `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. |
| 5.2 `forge gate --timeout-secs 1800` | PASS | The owning agent re-ran the Gate on the final tree: **exit 0**, `project: forge`, `freshness: fresh`, `aggregate: passed`, `profile: full`, `revision: 59ab6694c03c`, `observed_at: 2026-10-23T03:18:37Z`, `manifest_digest: sha256:2c456f0e82a570fb`, `rule_pack_version: forge-baseline/1.0.0`. All eight checks report `pass` — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests`. Two earlier Gate runs on the same tree were blocked on the `security` check alone, with `couldn't fetch advisory database … An IO error occurred when talking to the server`: the owning agent's at `observed_at: 2026-10-23T03:01:28Z` and the worker's at `2026-10-23T02:14:42Z`. Both were network faults to `github.com`, not findings — `cargo audit --no-fetch` against the cached advisory database exited 0 with `warning: 4 allowed warnings found`, `git ls-remote https://github.com/RustSec/advisory-db.git HEAD` answered once the network recovered, and the re-run passed all eight checks with `security` in state `pass`. The change added no failure. The worker's own passing Gate run on the same tree is recorded above at `observed_at: 2026-10-23T02:32:46Z`. The revision is the parent commit, because the implementation is still uncommitted in the worktree when the Gate runs; the Gate's own `repository` check ran against that worktree. |
| 5.3 strict validation | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 148 passed, 0 failed (148 items)`, the pre-change count. |
| 5.4 the change names | PASS | `node scripts/check-openspec-change-names.mjs` → `check-openspec-change-names: PASS`, exit 0. |
| 5.5 `git diff --check` | PASS | No output, exit 0. |

### Judgement calls

- **Only an explicit answer degrades the phase.** The canonical
  requirement names a provider that *reports* unhealthy. A row without
  a health is the absence of an answer, not an answer of `healthy`, and
  the projection must not invent a verdict from it.
- **The comparison is exact.** `healthy` is the adapter's own string.
  Anything else is not healthy.
- **An empty health is the absence of an answer.** The adapter contract
  makes `health` a required response field, so a provider that reports
  no health answers it empty. An empty value is not a health, so the
  row yields no health and keeps today's verdict.
- **The other branches are untouched.** The stage and preflight
  branches never produced `healthy`, so the provider's health does not
  change them.
- **The health is internal to the projection.** The per-verb report
  carries it, and the phase states it, but the delivery status
  contract keeps the fields it publishes: the new field is not
  serialized, so the projection's JSON keeps its shape and its
  contract version. The projection unit test
  `the_projection_json_keeps_its_published_fields` holds that.
- **The contract test drives the real workflow.** It runs the real
  `forge` binary through preflight, stage and promote against a stubbed
  provider on a controlled `PATH` — the pattern
  `tests/delivery_contract.rs` already uses — and then reads the real
  status projection. The stub reads the request on stdin, because the
  provider is invoked without arguments, and answers the promote verb
  with the build and run statuses the case needs.
