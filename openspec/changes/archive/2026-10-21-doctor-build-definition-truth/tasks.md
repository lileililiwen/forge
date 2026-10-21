# Tasks: doctor-build-definition-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the doctor's build-definition probes: `src/doctor/probes.rs` (`expected_build_file`, `has_build_definition`).
- [x] 1.3 Read the doctor's finding inventory: `src/doctor/runner.rs` (`profile-known`, `dependency-drift`, `build-config`).
- [x] 1.4 Read the doctor's maturity assessment: `src/doctor/assessment.rs` (the L1 structure and build controls).
- [x] 1.5 Read the supported profile catalogue: `src/profile/catalog.rs` and the generator's per-profile file list in `src/generate/engine.rs`.
- [x] 1.6 Read the canonical capabilities this change modifies: `doctor-maturity-assessment`.
- [x] 1.7 Read the house pinned-table pattern: `tests/workspace_metadata_contract.rs` and `tests/profile_build_command_truth_contract.rs`.

## 2. DFS requirement implementation

- [x] 2.1 Decide the expectation shape: a named file, a file extension, or one of several files.
- [x] 2.2 Decide the outcome shape: present, missing, and a profile the doctor cannot judge.
- [x] 2.3 Decide that a profile the doctor cannot judge is `UNAVAILABLE`, never a `FAIL` of the project.
- [x] 2.4 Decide the guard: the doctor's table is pinned against the supported catalogue, in both directions.

## 3. Implement

- [x] 3.1 `src/doctor/probes.rs`: `BuildDefinition`, `BuildDefinitionOutcome`, `expected_build_definition` and the rewritten `has_build_definition`; `react-web` expects `package.json`.
- [x] 3.2 `src/doctor/runner.rs`: the `build-config` and `dependency-drift` findings carry the three states.
- [x] 3.3 `src/doctor/assessment.rs`: the L1 structure and build controls read the new outcome and name an unjudgeable profile.
- [x] 3.4 `src/doctor/doctor_tests.rs`: the unit-level check for the three states.
- [x] 3.5 `tests/doctor_build_definition_contract.rs`: the pinned table against the catalogue, and the doctor's behaviour through the binary.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 `forge new --profile react-web` then `forge doctor` on the generated project: `build-config` is `PASS`.
- [x] 4.4 The same project without `package.json`: `build-config` is `FAIL` and the evidence names `package.json`.
- [x] 4.5 A project whose profile the doctor does not judge: `build-config` is `UNAVAILABLE`, not `FAIL`.
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

Section 6 was closed by the owning agent: the change was archived with
`openspec archive doctor-build-definition-truth --yes` (no `--skip-specs`),
so `openspec/specs/doctor-build-definition-truth/spec.md` was created and
`openspec/specs/doctor-maturity-assessment/spec.md` was updated. The
archive's `TBD` Purpose was replaced by hand with the paragraph the
capability's own requirements describe, and the modified capability's
Purpose was extended with the build-definition rule. The implementation and
its specs are committed together, and the handoff update is a separate
commit.

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed.

| Check | Result | Evidence |
|---|---|---|
| 4.1 `cargo build --bin forge` | pass | `cargo build --bin forge` → `Compiling forge v0.1.1 (/home/paul/code/forge)` / `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 39.48s`, exit code 0; the final confirmation run after the Gate reported `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 0.48s`, exit code 0. |
| 4.2 the repository test suite | pass | `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` → 211 test binaries ran (`Running` headers), the summed `test result` lines read `passed=4052 failed=0 ignored=1`, no `test result: FAILED` line anywhere in the log, `EXIT=0`. The new binary reported `Running tests/doctor_build_definition_contract.rs` → `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. |
| 3.5 `cargo test --test doctor_build_definition_contract` | pass | `running 5 tests` → `every_supported_profile_has_a_build_definition ... ok`, `the_doctors_table_matches_the_pinned_matrix_in_both_directions ... ok`, `a_healthy_react_project_passes_the_build_definition_checks ... ok`, `a_react_project_without_package_json_fails_the_build_definition ... ok`, `a_profile_the_doctor_cannot_judge_is_unavailable_not_failed ... ok`; `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s`. |
| 3.4 `cargo test -p forge --lib doctor::` | pass | `test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 1340 filtered out; finished in 0.03s`, including `doctor::doctor_tests::tests::build_definition_reads_present_missing_and_unknown_profile ... ok`. |
| 4.3 `forge new --profile react-web` then `forge doctor` | pass | `forge new /tmp/dbdt/app --profile react-web --id dbdt-app` → `created dbdt-app (/tmp/dbdt/app) from react-web@0.1.0`; `forge doctor /tmp/dbdt/app` → `[PASS] dependency-drift (automatic)` / `evidence: manifest runtime agrees with on-disk dependency evidence` and `[PASS] build-config (automatic)` / `evidence: package.json` / `detail: build definition is present`. The remaining non-pass findings are `[UNAVAILABLE] repository`, `[WARN] driftwatch-config` and `[UNAVAILABLE] driftwatch-policy` — none of them is the build definition. |
| 4.4 the same project without `package.json` | pass | `package.json` moved out of the project root, then `forge doctor /tmp/dbdt/app` → `[FAIL] build-config (automatic)` / `evidence: missing build definition 'package.json'` / `detail: build definition required by L1 is missing`, and `[WARN] dependency-drift (automatic)` / `evidence: missing build definition 'package.json'`. |
| 4.5 a profile the doctor does not judge | pass | A hand-written `forge.yaml` naming `profile: flutter-client`, then `forge doctor /tmp/dbdt/planned` → `[UNAVAILABLE] build-config (automatic)` / `evidence: the doctor has no expected build definition for profile 'flutter-client'` / `detail: required inspector (build definition) cannot run`, and `[UNAVAILABLE] dependency-drift (manual)` with the same evidence. The finding is not `FAIL` and never says "unknown profile". |
| 4.6 `rustfmt --edition 2021 --check` on every touched Rust file | pass | `rustfmt --edition 2021 --check` exited 0 for each of `src/doctor/probes.rs`, `src/doctor/runner.rs`, `src/doctor/assessment.rs`, `src/doctor/doctor_tests.rs` and `tests/doctor_build_definition_contract.rs` (no diff output). `git status --short` over `crates/cargo-audit-boost`, `crates/cargo-sonar`, `src/cli/use_cmd.rs`, `tests/delivery_cross_surface.rs` and `tests/forge_web_project_delivery_contract.rs` printed nothing: no pre-existing rustfmt-dirty file was touched. |
| 4.7 `cargo clippy --workspace --all-targets --all-features` | pass | exit code 0, 0 `error` lines. 39 lines start with `warning` (29 lint warnings plus 10 per-target summary lines) — the pre-existing baseline. No warning points at `src/doctor/` (`grep -E '^\s+--> ' … \| grep -c 'src/doctor/'` → 0). The warnings sit in `crates/cargo-sonar/src/analyzer.rs` (10), `crates/cargo-audit-boost/src/ui.rs` (3), `crates/cargo-sonar/src/main.rs` (2), `crates/cargo-audit-boost/src/parser.rs` (2), `crates/cargo-audit-boost/src/main.rs` (2), `crates/cargo-audit-boost/src/model.rs`, `crates/cargo-audit-boost/src/exporter.rs`, `src/registry/model.rs`, `src/hosted_ci/store.rs`, `src/deploy/model.rs`, `src/contract/mod.rs`, `src/cli/auth.rs`, `src/auth/setup.rs`, `tests/origin_repository_retirement_contract.rs` and `tests/deploy_ssh_adapter_contract.rs`. |
| 4.8 the source file line counts of every touched file | pass | `src/doctor/probes.rs` 931, `src/doctor/runner.rs` 532, `src/doctor/assessment.rs` 595, `src/doctor/doctor_tests.rs` 878, `tests/doctor_build_definition_contract.rs` 223 — every one is under the 1000 physical line limit. |
| 5.1 `forge gate --dry-run` | pass | `dry-run: gate plan rehearsal for forge (nothing was executed, persisted or journaled)`; `checks (8 planned):` — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests`, every one `[required]`; `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`; `unbound required checks: none`; exit code 0. |
| 5.2 `forge gate --timeout-secs 1800` | pass | `project: forge` / `freshness: fresh` / `aggregate: passed` / `profile: full` / `runtime: forge (forge 0.1.1)` / `revision: 622e8d92fbb2` / `observed_at: 2026-10-21T22:14:13Z` / `manifest_digest: sha256:2c456f0e82a570fb` / `rule_pack_version: forge-baseline/1.0.0`. All eight checks are `pass`: `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests`. `persisted: .forge/gate/forge/evidence.json`, exit code 0. The persisted record carries `aggregate: passed`, `revision: 622e8d92fbb282d8aa4f34413db5774e3590cd53`, `observed_at: 2026-10-21T22:14:13Z` and `state: pass` for every check. The revision is the current HEAD commit: the implementation is still uncommitted in the worktree, and the Gate's own `repository` check (`git diff --check`) ran against that worktree. The tree was warmed first with `cargo build --workspace` and `cargo test --workspace --no-run`, so the 60 s `build` check did not lose the shared build lock. |
| 5.3 `openspec validate --all --strict --no-interactive` | pass | `node /home/paul/.nvm/versions/node/v24.18.0/lib/node_modules/@fission-ai/openspec/bin/openspec.js validate --all --strict --no-interactive` → `✓ change/doctor-build-definition-truth`, `✓ spec/doctor-maturity-assessment`, `Totals: 139 passed, 0 failed (139 items)`, exit code 0. |
| 5.4 `node scripts/check-openspec-change-names.mjs` | pass | `check-openspec-change-names: PASS`, exit code 0. |
| 5.5 `git diff --check` | pass | no output, exit code 0. |
