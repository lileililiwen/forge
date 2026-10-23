# Tasks: release-driftwatch-check-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the release lane's DriftWatch check: `src/release/engine/prepare.rs` (`run_driftwatch_check`) and the readiness rule (`check_readiness`).
- [x] 1.3 Read the release lane's doctor check in the same file, as the sibling that already judges with the doctor's blocking set.
- [x] 1.4 Read the doctor's policy judgement: `src/doctor/assessment.rs` (`policy_findings`, `reported_policy_findings`, `aggregate_rollup`) and `src/doctor/freshness.rs` (`observation_is_stale_from_report`).
- [x] 1.5 Read the policy model: `src/policy/model.rs` (`PolicyReport`, `PolicyFinding`, `PolicySeverity`, `PolicyOutcome`).
- [x] 1.6 Read the canonical capabilities: `openspec/specs/release-publishing/spec.md` ("Verified release preparation") and `openspec/specs/quality-policy-integration/spec.md` ("Delegated policy execution").
- [x] 1.7 Reproduce the three disagreeing reports with a fake policy adapter (`FORGE_DRIFTWATCH_BIN`) and record the doctor and release-lane verdicts for each.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the doctor's judgement of a policy report becomes one typed verdict — `Failing`, `Stale`, `Unjudged`, `Warning`, `Passed` — with the doctor's own precedence, and the release lane reads it.
- [x] 2.2 Decide the mapping: `Failing` → `fail`, `Stale` → `stale`, `Unjudged` → `unavailable`, `Warning` → `pass` with the warnings named, `Passed` → `pass` with the report called healthy.
- [x] 2.3 Decide the warning case: a warning does not block, because the canonical release requirement names three blocking conditions and a warning is none of them; what changes is the evidence, which stops calling a warninging report healthy.
- [x] 2.4 Decide the evidence: a captured check names the rules behind its verdict, and the word *healthy* is reserved for a report where every applicable rule passed.
- [x] 2.5 Decide the guard: the release lane's five verdicts are pinned by a contract check that drives the real plan.

## 3. Implement

- [x] 3.1 `src/doctor/assessment.rs`: the policy verdict is a typed value; the rollup is expressed through it, and the doctor's statuses and wording stay exactly as they are.
- [x] 3.2 `src/doctor/assessment.rs`: the unit tests for the verdict.
- [x] 3.3 `src/release/engine/prepare.rs`: the DriftWatch check reads the verdict and captures the matching status and evidence.
- [x] 3.4 `src/release/engine/prepare.rs`: the release lane's doctor check, test check, Gate check and the readiness rule are untouched.
- [x] 3.5 `tests/release_driftwatch_check_truth_contract.rs` (new): the five pinned verdicts, driven through the public `forge::release::engine::prepare_release`.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 The release plan for a policy report with a failing rule: the check is `fail` and the plan is not ready.
- [x] 4.4 The release plan for a policy report with no applicable rule: the check is `unavailable` and the plan is not ready.
- [x] 4.5 The release plan for a policy report from an older revision: the check is `stale` and the plan is not ready.
- [x] 4.6 The release plan for a policy report that warned: the check is `pass`, the evidence names the warning, and the evidence does not say *healthy*.
- [x] 4.7 The release plan for a policy report where every rule passed: the check is `pass` with `driftwatch report healthy`.
- [x] 4.8 The doctor on the same five reports: its statuses and wording are unchanged.
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
| 1.7 the three disagreeing reports | PASS | Reproduced on this host before the change with a fake policy adapter (`FORGE_DRIFTWATCH_BIN` pointing at a script that answers `--version` and prints a legacy adapter report) and one project declaring `release.checks: [driftwatch]`. **Before**, all three reports produced the same captured check: `driftwatch pass applicable=true revision=…: driftwatch report is healthy`. **After**: the warning report is `driftwatch pass … driftwatch report carries warnings`, the unjudged report is `driftwatch unavailable … driftwatch applied no rule; there is no policy evidence to release on`, and the stale report is `driftwatch stale … driftwatch report is stale relative to the current source revision`. The same three reports through `forge doctor` read `[WARN] driftwatch-policy`, `[UNAVAILABLE] driftwatch-policy` and `[WARN] driftwatch-policy` (stale) — the doctor's verdicts are unchanged. |
| 4.2 the repository test suite | PASS | `bash scripts/check-tests.sh` — the Gate's own `tests` command, and the same suite CI's `test` job runs — exited 0. The 223 `test result:` lines sum to **4148 passed; 0 failed; 1 ignored**; `grep -c FAILED` over the whole log returns 0. The pre-change baseline was 4136 passed over 222 lines, so this change adds exactly the 12 tests it declares (11 verdict cases in `src/doctor/assessment.rs` + 1 contract case) and exactly one test binary. The one ignored test is `tampering::packing_leaves_the_sibling_checkout_byte_identical`. Neither known flake fired: `checker_contract::format_json_does_not_alter_checker_stdout` and the Flutter-SDK crash in `profile_build_command_runnable_contract` both passed in this run, so no re-run was needed. |
| 3.5 the contract test | PASS | `cargo test --test release_driftwatch_check_truth_contract` → `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.33s`. It drives the real plan through the public `forge::release::engine::prepare_release` with the fake adapter and pins the five verdicts on both surfaces at once. `cargo test -p forge --lib doctor::` → `test result: ok. 67 passed; 0 failed; 0 ignored; 0 measured; 1363 filtered out`; `cargo test -p forge --lib release::` → `test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 1396 filtered out`. |
| 4.3 the failing report | PASS | A report whose applicable rule carries `fail` severity. **Before**: `driftwatch pass … driftwatch report is healthy`, and the plan did not name the rule. **After**: `driftwatch fail applicable=true revision=…: driftwatch reports failing rules`, with the failing rule id in the evidence, and the plan's summary names `failing check(s) driftwatch:fail`. |
| 4.4 the unjudged report | PASS | A report whose only rule has `applicable: false`. **Before**: `driftwatch pass … driftwatch report is healthy` — the plan was ready to release on a report the doctor called `[UNAVAILABLE]`. **After**: `driftwatch unavailable applicable=true revision=…: driftwatch applied no rule; there is no policy evidence to release on`, and the plan's summary reads `release plan not ready: failing check(s) driftwatch:unavailable, gate:fail; release execution blocks before tag or publication`. |
| 4.5 the stale report | PASS | A report with `"source_revision": "2020-01-01T00:00:00Z"`. **Before**: `driftwatch pass … driftwatch report is healthy`. **After**: `driftwatch stale applicable=true revision=…: driftwatch report is stale relative to the current source revision`, and the plan's summary reads `failing check(s) driftwatch:stale, gate:fail; release execution blocks before tag or publication`. |
| 4.6 the warning report | PASS | A report whose applicable rule carries `warn` severity. **After**: `driftwatch pass applicable=true revision=…: driftwatch report carries warnings` — the evidence names the warning (`driftwatch warned on rule(s) POLICY-DOCS-001`) and the word *healthy* appears nowhere in the check. The check still passes: a warning is not one of the three blocking conditions the canonical release requirement names, and the release lane's own doctor check does not block on a warning finding either. |
| 4.7 the passing report | PASS | A report whose applicable rule carries `pass` severity. **After**: `driftwatch pass applicable=true revision=…: driftwatch report is healthy` — the only case that says *healthy* — and the doctor reads `[PASS] driftwatch-policy` with `1 finding(s): 1 pass, 0 warn, 0 fail`. |
| 4.8 the doctor's unchanged verdicts | PASS | `forge doctor` on the same four reports: the warning report → `[WARN] driftwatch-policy … 1 finding(s): 0 pass, 1 warn, 0 fail`; the unjudged report → `[UNAVAILABLE] driftwatch-policy … 1 finding(s): 0 pass, 0 warn, 0 fail`; the stale report → `[WARN] driftwatch-policy … source revision older than current forge.yaml`; the passing report → `[PASS] driftwatch-policy … 1 finding(s): 1 pass, 0 warn, 0 fail`. All four are byte-identical to the pre-change output. The unit test `the_rollup_keeps_the_doctors_own_statuses` and `the_rollup_keeps_the_doctors_own_counts_and_wording` pin the rollup's statuses, counts and wording. |
| 4.9 rustfmt | PASS | `rustfmt --edition 2021 --check` prints nothing for `src/doctor/assessment.rs`, `src/doctor/freshness.rs`, `src/release/engine/prepare.rs` and `tests/release_driftwatch_check_truth_contract.rs` (all clean). `cargo fmt --all` was never run. `git status --porcelain` shows only the owner's `M HANDOFF.md`, the three modified source files, the new contract test and the change package; none of the twelve pre-existing rustfmt-dirty files (`crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `src/cli/use_cmd.rs`, `tests/delivery_cross_surface.rs`, `tests/forge_web_project_delivery_contract.rs`) is touched. |
| 4.10 clippy | PASS | `cargo clippy --workspace --all-targets --all-features` exited 0 with **39** warnings — the pre-existing baseline, unchanged. `grep "doctor/assessment.rs\|doctor/freshness.rs\|release/engine/prepare.rs\|release_driftwatch_check_truth_contract.rs"` over the clippy log returns nothing: no warning in any touched file. |
| 4.11 the source file line counts | PASS | `wc -l src/doctor/assessment.rs` → 878; `src/doctor/freshness.rs` → 223; `src/release/engine/prepare.rs` → 511; `tests/release_driftwatch_check_truth_contract.rs` → 409. All four are under the 1000 physical line limit. |
| 5.1 `forge gate --dry-run` | PASS | Exit 0: `checks (8 planned)` — build, declared-verification, placeholder-threshold, product-code-boundary, repository, security, source-file-size, tests — with `unbound required checks: none` and `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. Nothing was executed, persisted or journaled. |
| 5.2 `forge gate --timeout-secs 1800` | PASS | Exit 0: `project: forge`, `freshness: fresh`, `aggregate: passed`, `profile: full`, `revision: 43065cba34b4`, `observed_at: 2026-10-23T00:18:32Z`, `rule_pack_version: forge-baseline/1.0.0`. All eight checks report `pass`, including `source-file-size pass: evaluated 458 of 458 candidate file(s); maximum 1000 physical line(s)` and `build pass: diagnostic: Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 0.20s` — the build was warm before the run. Evidence persisted to `.forge/gate/forge/evidence.json` (gitignored; `git status` stays clean). The revision is the parent commit, because the implementation is still uncommitted when the Gate runs. A first Gate run on the same tree failed on the `security` check alone, with `couldn't fetch advisory database … An IO error occurred when talking to the server` — `cargo audit` could not reach the RustSec advisory database. It was a network fault, not a defect: `cargo audit` run on its own then exited 0 with `warning: 4 allowed warnings found`, `git ls-remote https://github.com/RustSec/advisory-db.git HEAD` answered, and the second Gate run passed all eight checks with the `security` check in state `pass`. The change added no failure. |
| 5.3 strict validation | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 146 passed, 0 failed (146 items)`, including `✓ change/release-driftwatch-check-truth`, `✓ spec/release-publishing` and `✓ spec/quality-policy-integration`. |
| 5.4 the change names | PASS | `node scripts/check-openspec-change-names.mjs` → `check-openspec-change-names: PASS`. |
| 5.5 `git diff --check` | PASS | No output (no whitespace error). |

### Judgement calls

- **The rule stays in the doctor.** The doctor already judges a policy
  report, and the release lane is already a consumer of the doctor. A
  typed verdict on the doctor's side is one rule in one place; the
  release lane reads it instead of re-deriving it.
- **The doctor's own behaviour does not change.** The verdict expresses
  the rule the rollup already implements, including the case where a
  report is both stale and unjudged.
- **A warning does not block.** The canonical release requirement names
  three blocking conditions and a warning is none of them; the release
  lane's own doctor check does not block on a warning finding either.
- **The contract test drives the real plan.** It calls the public
  `forge::release::engine::prepare_release` with a fake policy adapter
  — the same `FORGE_DRIFTWATCH_BIN` script pattern as
  `tests/workbench_health_latency_contract.rs` — so no public seam is
  widened and no private module is opened.
