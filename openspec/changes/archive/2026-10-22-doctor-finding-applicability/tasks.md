# Tasks: doctor-finding-applicability

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the doctor's finding inventory: `src/doctor/runner.rs` (`ci-config`, `deployment-config`, `driftwatch-config` and how the report is assembled).
- [x] 1.3 Read the doctor's maturity controls: `src/doctor/assessment.rs` (`at_least`, the L2-ci, L2-deployment and L2-driftwatch controls).
- [x] 1.4 Read the doctor's contract: `src/doctor/mod.rs` and `src/doctor/model.rs` (the finding model, its `applicable` flag and the statuses).
- [x] 1.5 Read the release lane's DriftWatch row: `src/release/engine/prepare.rs` and the manifest's release-checks model.
- [x] 1.6 Read the checker plane: the code behind `forge check`.
- [x] 1.7 Read the canonical capabilities this change modifies: `doctor-maturity-assessment`.

## 2. DFS requirement implementation

- [x] 2.1 Decide the applicability rule: the target maturity reaches the level that declares the mirrored control.
- [x] 2.2 Decide the DriftWatch exception: the manifest's release checks declare a DriftWatch check, which is the rule the release lane already applies.
- [x] 2.3 Decide the shape of a configuration the project does not require: reported as not applicable, with its evidence kept, and never rendered as a warning.
- [x] 2.4 Decide the guard: the applicability of the three findings is pinned across the target maturities.

## 3. Implement

- [x] 3.1 `src/doctor/runner.rs`: `ci-config`, `deployment-config` and `driftwatch-config` apply the shared rule and report a configuration the project does not require as not applicable.
- [x] 3.2 `src/doctor/runner.rs`: the DriftWatch finding also applies when the manifest's release checks declare a DriftWatch check.
- [x] 3.3 `src/doctor/doctor_tests.rs`: the case-level checks for the three findings.
- [x] 3.4 `tests/doctor_finding_applicability_contract.rs`: the pinned applicability of the three findings across the target maturities.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 `forge doctor` on an L1 project: the three configuration findings are not applicable, and the report agrees with its own control list.
- [x] 4.4 `forge doctor` on an L2 project without the configurations: the three findings warn, as before.
- [x] 4.5 `forge check` on the L1 project: no configuration alert.
- [x] 4.6 A project whose manifest declares the driftwatch check at L1: the DriftWatch finding warns.
- [x] 4.7 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.8 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.9 Every touched file stays under the 1000 physical line limit.

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

Closed by the owning agent: the change was archived with `openspec archive
doctor-finding-applicability --yes` (no `--skip-specs`), so
`openspec/specs/doctor-finding-applicability/spec.md` was created and
`openspec/specs/doctor-maturity-assessment/spec.md` was updated with every
canonical scenario kept. The archive's `TBD` Purpose was replaced by hand
with the paragraph the capability's own requirement describes, and the
modified capability's Purpose now states that the target-gated
applicability is shared by each configuration finding and the control it
mirrors. The implementation and its specs are committed together, and the
handoff update is a separate commit.

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed.

| Check | Result | Evidence |
|---|---|---|
| 4.1 `cargo build --bin forge` | PASS | `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 23.70s` (exit 0) |
| 4.2 the repository test suite | PASS | `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` → 214 test binaries, **4068 passed, 1 failed, 1 ignored**. The single failure is `profile_build_command_runnable_contract::every_declared_build_command_runs_in_a_generated_project`: the Flutter SDK crashed while resolving packages (`Crash! The Dart VM service is not available` / `flutter build bundle` → `Unhandled exception: Crash!`); it is unrelated to the doctor and passes on a re-run — `cargo test --test profile_build_command_runnable_contract` → `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. The Gate's own `tests` check, which runs the suite again, passed (5.2). |
| 3.4 `cargo test --test doctor_finding_applicability_contract` | PASS | `running 5 tests` … `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| 3.3 `cargo test -p forge --lib doctor::` | PASS | `running 50 tests` … `test result: ok. 50 passed; 0 failed; 0 ignored; 0 measured; 1340 filtered out` |
| 4.3 `forge doctor` on an L1 project | PASS | **Before** (`forge doctor /tmp/forge-doctor-fixtures/l1-app`): `[WARN] deployment-config (manual)` / `[WARN] ci-config (automatic)` / `[WARN] driftwatch-config (automatic)` with `detail: no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3)`, while the same report's control list printed `[N/A] L2-driftwatch (L2): driftwatch is configured; execution belongs to v0.3 (L2 DriftWatch)`. **After**: `[PASS] deployment-config (manual)` / `evidence: no deployment configuration` / `detail: the project does not require a deployment configuration at its target maturity` / `applicability: not applicable`; `[PASS] ci-config (automatic)` / `evidence: no ci configuration` / `detail: the project does not require a CI configuration at its target maturity` / `applicability: not applicable`; `[PASS] driftwatch-config (automatic)` / `evidence: no driftwatch configuration` / `detail: the project does not require a DriftWatch configuration at its target maturity` / `applicability: not applicable`. The JSON report agrees with its own control list: `finding driftwatch-config | applicable: False | status: pass | evidence: ['no driftwatch configuration']` and `control L2-driftwatch | applicable: False | met: False | ['no driftwatch configuration']` (same for `L2-ci` and `L2-deployment`). |
| 4.4 `forge doctor` on an L2 project | PASS | `forge doctor /tmp/forge-doctor-fixtures/l2-app` (manifest `maturity: L2`, `target_maturity: L2`, no configuration files): `finding deployment-config \| applicable: True \| status: warn \| evidence: ['no deployment configuration'] \| detail: no deployment configuration; required for L2 deployment`; `finding ci-config \| applicable: True \| status: warn \| evidence: ['no ci configuration'] \| detail: no ci configuration; required for L2 ci`; `finding driftwatch-config \| applicable: True \| status: warn \| evidence: ['no driftwatch configuration'] \| detail: no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3)` — today's status and detail, unchanged. |
| 4.5 `forge check` on the L1 project | PASS | **Before**: `{"severity":"warning","message":"driftwatch-config: no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3); no driftwatch configuration","source":"doctor","symbol":"doctor/driftwatch-config"}` (plus `doctor/deployment-config` and `doctor/ci-config`). **After**: `forge check /tmp/forge-doctor-fixtures/l1-app` → `{"schema":"forge-checker/0.1.0",…,"alerts":[{"severity":"warning","message":"repository: required inspector (git repository) cannot run; not a git repository; remote cannot be determined","source":"doctor","symbol":"doctor/repository"}]}` — no configuration alert. The L2 project still reports all three: `doctor/deployment-config`, `doctor/ci-config`, `doctor/driftwatch-config`. |
| 4.6 `forge doctor` on a project that declares the driftwatch check | PASS | `forge doctor /tmp/forge-doctor-fixtures/dw-checked-app` (manifest `maturity: L1`, `target_maturity: L1`, `release.checks: [- kind: driftwatch]`): `finding driftwatch-config \| applicable: True \| status: warn \| evidence: ['no driftwatch configuration'] \| detail: no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3)`, while `finding ci-config \| applicable: False \| status: pass` and `finding deployment-config \| applicable: False \| status: pass` stay not applicable. |
| 4.7 `rustfmt --edition 2021 --check` on every touched Rust file | PASS | `src/doctor/runner.rs` → `rustfmt-clean`; `src/doctor/doctor_tests.rs` → `rustfmt-clean`; `tests/doctor_finding_applicability_contract.rs` → `rustfmt-clean`; `tests/driftwatch_cli_contract.rs` → `rustfmt-clean`; `tests/driftwatch_cli_alignment_contract.rs` → `rustfmt-clean`; `tests/checker_contract.rs` → `rustfmt-clean`. `src/doctor/runner.rs` and `src/doctor/doctor_tests.rs` were verified rustfmt-clean at `HEAD` before the change, so only the new lines were formatted. `git status --porcelain` shows no pre-existing rustfmt-dirty file (`crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `src/cli/use_cmd.rs`, `tests/delivery_cross_surface.rs`, `tests/forge_web_project_delivery_contract.rs`) among the modified paths. |
| 4.8 `cargo clippy --workspace --all-targets --all-features` | PASS | exit code 0; **39 warnings**, the pre-existing baseline. No warning is in a touched file — the warning locations are `crates/cargo-sonar/src/analyzer.rs` (10), `crates/cargo-audit-boost/src/ui.rs` (3), `crates/cargo-sonar/src/main.rs` (2), `crates/cargo-audit-boost/src/parser.rs` (2), `crates/cargo-audit-boost/src/main.rs` (2), `crates/cargo-audit-boost/src/model.rs` (1), `crates/cargo-audit-boost/src/exporter.rs` (1), `tests/origin_repository_retirement_contract.rs` (1), `tests/deploy_ssh_adapter_contract.rs` (1), `src/registry/model.rs` (1), `src/hosted_ci/store.rs` (1), `src/deploy/model.rs` (1), `src/contract/mod.rs` (1), `src/cli/auth.rs` (1), `src/auth/setup.rs` (1). |
| 4.9 the source file line counts of every touched file | PASS | `src/doctor/runner.rs` 651; `src/doctor/doctor_tests.rs` 987; `tests/doctor_finding_applicability_contract.rs` 254; `tests/driftwatch_cli_contract.rs` 476; `tests/driftwatch_cli_alignment_contract.rs` 584; `tests/checker_contract.rs` 594 — all under 1000. The Gate's `source-file-size` check agrees: `evaluated 456 of 456 candidate file(s); maximum 1000 physical line(s)`. |
| 5.1 `forge gate --dry-run` | PASS | exit 0; `checks (8 planned):` — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests`; `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`; `unbound required checks: none`. |
| 5.2 `forge gate --timeout-secs 1800` | PASS | exit 0; `aggregate: passed`; `revision: 63a00b8589406444c959a97fa7257d7466da8e59`; `observed_at: 2026-10-22T01:16:13Z`; `freshness: fresh`; `profile: full`; `manifest_digest: sha256:2c456f0e82a570fb`; `rule_pack_version: forge-baseline/1.0.0`. All eight checks are `state: pass` in `.forge/gate/forge/evidence.json`: `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests`. |
| 5.3 `openspec validate --all --strict --no-interactive` | PASS | `Totals: 142 passed, 0 failed (142 items)`; `doctor-finding-applicability` and `doctor-maturity-assessment` are valid with the new requirement and the modified capability. |
| 5.4 `node scripts/check-openspec-change-names.mjs` | PASS | `check-openspec-change-names: PASS` (exit 0) |
| 5.5 `git diff --check` | PASS | no output, exit 0 |

### 7.1 Scope note: three pre-existing contract tests carried the old behaviour

The change makes a configuration the project does not require stop
warning. Three existing contract tests encoded that warning as their
fixture, so they failed against the new contract and were updated to it.
No production file outside `src/doctor/runner.rs` was touched, and the
checker plane was not modified.

| File | Observed failure before the update | What changed |
|---|---|---|
| `tests/driftwatch_cli_contract.rs` | `doctor_recognizes_the_real_driftwatch_markers` panicked at line 437: `assertion 'left == right' failed / left: String("pass") / right: "warn"` — the L1 fixture's `driftwatch-config` is now not applicable | the assertion now pins `applicable: false` and `status: "pass"`; the evidence assertion is unchanged |
| `tests/driftwatch_cli_alignment_contract.rs` | `doctor_recognizes_sibling_markers` panicked at line 407: `assertion 'left == right' failed / left: String("pass") / right: "warn"` — same cause | same update; the "absence is its own distinct state" check stays on the evidence |
| `tests/checker_contract.rs` | `truncation_is_bounded_and_explicit` panicked at line 365: `fixture must produce warnings: []` — dropping the three configurations from an L1 project no longer produces warnings, so the fixture had none to truncate | the fixture's project now targets L2 (`clean_project_at`), the level at which the three dropped configurations are required and warn; the truncation assertions are unchanged |

After the updates all three binaries are green:
`tests/checker_contract.rs` → `test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`;
`tests/driftwatch_cli_alignment_contract.rs` → `test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`;
`tests/driftwatch_cli_contract.rs` → `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
