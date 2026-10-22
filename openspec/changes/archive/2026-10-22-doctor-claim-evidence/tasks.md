# Tasks: doctor-claim-evidence

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the doctor's finding inventory: `src/doctor/runner.rs` (the `registry-observation` and `dependency-drift` findings, and how the report is assembled).
- [x] 1.3 Read the doctor's maturity controls: `src/doctor/assessment.rs` (`at_least`, the `L1-structure` and `L3-registry` controls).
- [x] 1.4 Read the doctor's contract: `src/doctor/mod.rs` and `src/doctor/model.rs` (the finding model, its `applicable` flag and the statuses).
- [x] 1.5 Read the registry callers: `src/cli/projects.rs` and `src/mcp/params.rs` (`observation_for`), and the API surfaces that always supply a registered observation.
- [x] 1.6 Read the release lane's doctor check: `src/release/engine/prepare.rs` (the caller that supplies no observation).
- [x] 1.7 Read the checker plane: the code behind `forge check`.
- [x] 1.8 Read the canonical capabilities this change modifies: `doctor-maturity-assessment`.

## 2. DFS requirement implementation

- [x] 2.1 Decide the registry observation's three states: a fresh observation, an observation that says the project is not registered, and the absence of an observation reported as unknown and not applicable.
- [x] 2.2 Decide the callers' half: a caller that consulted the registry and found no record supplies an observation that says so, so the genuine warning is kept.
- [x] 2.3 Decide the `L3-registry` control: applicable only when the doctor holds an observation, and naming the unknown when it does not.
- [x] 2.4 Decide the guard: the claims are pinned across the registry states, the callers, the structure control and the drift evidence.

## 3. Implement

- [x] 3.1 `src/doctor/runner.rs`: the `registry-observation` finding reports the unknown state as not applicable when no observation is supplied, and the `dependency-drift` PASS evidence names the manifest's and the profile's language.
- [x] 3.2 `src/doctor/assessment.rs`: the `L1-structure` requirement text names the manifest and the build definition, and the `L3-registry` control follows the observation.
- [x] 3.3 `src/cli/projects.rs` and `src/mcp/params.rs`: an unregistered project supplies an observation that says the project is not registered.
- [x] 3.4 `src/doctor/claim_evidence_tests.rs`: the case-level checks for the registry states, the registration control, the structure control and the drift evidence. The checks live in a sibling module rather than in `src/doctor/doctor_tests.rs`, because that file already held 987 physical lines and the repository Gate's `source-file-size` check caps every file at 1000; the house already splits doctor test modules this way (`src/doctor/delivery_tests.rs`).
- [x] 3.5 `tests/doctor_claim_evidence_contract.rs`: the pinned claims.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 `forge doctor` on a project: the drift evidence names both languages, and the structure control names the two checks it performs.
- [x] 4.4 The release lane's doctor run: the registry observation reports the unknown state instead of claiming the project is unregistered.
- [x] 4.5 `forge doctor` for a project that is genuinely unregistered: the warning stays.
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

Closed by the owning agent: the change was archived with `openspec archive
doctor-claim-evidence --yes` (no `--skip-specs`), so
`openspec/specs/doctor-claim-evidence/spec.md` was created with four
requirements and `openspec/specs/doctor-maturity-assessment/spec.md` was
updated with every canonical scenario of the "Doctor finding inventory"
requirement kept and one scenario added. The archive's `TBD` Purpose was
replaced by hand with the paragraph the capability's own requirements
describe, and the modified capability's Purpose now states that the
doctor's statements name only the checks it performed and the facts it
read. The implementation and its specs are committed together, and the
handoff update is a separate commit.

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed.

| Check | Result | Evidence |
|---|---|---|
| 4.1 `cargo build --bin forge` | PASS | `cargo build --bin forge` exited 0: `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 58.27s`. `cargo build --workspace` also exited 0: `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 5.40s`. |
| 4.2 the repository test suite | PASS | The worker's run: `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` exited 0; the `test result:` lines sum to **4083 passed, 0 failed, 1 ignored**. The owning agent's own run over the same tree (after the machine's `/tmp` and `target/` were wiped and the workspace was rebuilt from scratch) exited 101 with **4082 passed, 1 failed, 1 ignored** across 215 binaries. The single failure is `checker_contract::format_json_does_not_alter_checker_stdout`, and it is a pre-existing timing flake unrelated to this change: the test runs `forge check` twice and compares the two stdout bytes after stripping only the `"generated_at":"` marker, so two runs that straddle a second boundary differ (`left: "{\"schema\":\"forge-checker/0.1.0\",2026-10-22T05:56:57Z\",\"alerts\":[]}"` / `right: "…05:56:58Z…"`). Nothing in the change touches a timestamp, and the fixture is a clean project with zero alerts. The binary passes on re-run: `cargo test --test checker_contract` → `test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`, and the single case alone passed three times in a row. The Gate's own `tests` check, which runs the same suite, passed (5.2). |
| the change's own new tests | PASS | `cargo test --test doctor_claim_evidence_contract` → `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` (the eight cases: `a_doctor_run_without_an_observation_does_not_claim_the_project_is_unregistered`, `an_observation_that_says_not_registered_still_warns`, `the_cli_reports_an_unregistered_project_as_not_registered`, `the_mcp_surface_reports_an_unregistered_project_as_not_registered`, `the_l3_registry_control_is_applicable_only_with_an_observation`, `the_l1_structure_control_names_the_checks_it_performs`, `the_dependency_drift_evidence_names_both_languages`, `a_manifest_without_a_runtime_language_says_no_language_was_compared`). `cargo test -p forge --lib doctor::` → `test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 1340 filtered out`, including the six new `doctor::claim_evidence_tests::tests::` cases, all `ok`. |
| 3.4 `cargo test --test doctor_claim_evidence_contract` | PASS | `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s`. |
| 3.5 `cargo test -p forge --lib doctor::` | PASS | `test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 1340 filtered out`. The six new cases are listed as `doctor::claim_evidence_tests::tests::{no_registry_observation_leaves_the_registration_state_unknown, an_observation_that_says_not_registered_still_warns, the_l3_registry_control_follows_the_observation, the_l1_structure_control_names_the_checks_it_performs, the_dependency_drift_evidence_names_both_languages, a_manifest_without_a_runtime_language_says_no_language_was_compared}`, all `ok`. |
| 4.3 `forge doctor` on a project | PASS | `forge new /tmp/dce21/rust-web-app --profile rust-web --registry /tmp/dce21/registry.db`, then `forge --registry /tmp/dce21/registry.db doctor /tmp/dce21/rust-web-app` printed: `[PASS] dependency-drift (automatic)` / `evidence: manifest runtime language 'rust' agrees with profile language 'rust'` / `evidence: the profile's build definition is present (Cargo.toml)` / `detail: no dependency drift detected`, and `[met] L1-structure (L1): the project carries its manifest and its profile's build definition (L1 structure)` / `evidence: forge.yaml and profile build definition are present`. |
| 4.4 the release lane's doctor run | PASS | The release lane's check is `run_doctor(project_dir, Some(target), None, None)` (`src/release/engine/prepare.rs:275`) — it supplies no observation. The owning agent drove the lane: `forge new /tmp/dce21/rust-web-app --profile rust-web --registry /tmp/dce21/registry.db`, then `git init` + one commit in that project, then `forge --registry /tmp/dce21/registry.db release prepare --version 0.1.2 --dry-run --format json` recorded the doctor check as `{"kind": "doctor", "status": "pass", "applicable": true, "evidence": ["doctor healthy at target maturity"], "detail": "doctor is healthy at the configured target maturity"}`. The same no-observation state seen from the CLI — `forge --registry /tmp/dce21/not-a-registry doctor /tmp/dce21/rust-web-app`, with a registry path that is a directory and therefore cannot be opened — printed `[PASS] registry-observation (automatic)` / `evidence: no registry observation was supplied; the registration state is unknown to the doctor` / `detail: no registry observation was supplied; the doctor does not open the registry itself` / `applicability: not applicable`. The doctor no longer claims the project is unregistered. (A registry path that is a *file* the CLI can create is opened successfully, so it produces a real observation and the "not registered" warning — that is the 4.5 case, not this one.) |
| 4.5 `forge doctor` for an unregistered project | PASS | `forge --registry /tmp/dce21/registry.db doctor /tmp/dce21/unregistered` (a project the registry does not hold) printed `[WARN] registry-observation (automatic)` / `evidence: project is not registered; maturity evidence is local-only` / `detail: project is not registered`. The warning stays, and it is now backed by the registry read the caller actually made. |
| 4.6 `rustfmt --edition 2021 --check` on every touched Rust file | PASS | The owning agent re-ran `rustfmt --edition 2021 --check` over every touched Rust file after the worker's run and found `src/doctor/claim_evidence_tests.rs` not clean (one `assert!` argument was wrapped differently); the file was formatted with `rustfmt --edition 2021 src/doctor/claim_evidence_tests.rs` and a re-check reports `rustfmt-clean` for all seven touched Rust files: `src/doctor/runner.rs`, `src/doctor/assessment.rs`, `src/doctor/mod.rs`, `src/doctor/claim_evidence_tests.rs`, `src/cli/projects.rs`, `src/mcp/params.rs`, `tests/doctor_claim_evidence_contract.rs`. `git status --porcelain` shows the modified paths `docs/quickstart.md`, `src/cli/projects.rs`, `src/doctor/assessment.rs`, `src/doctor/mod.rs`, `src/doctor/runner.rs`, `src/mcp/params.rs` and the two new files; none of the 12 pre-existing rustfmt-dirty files (`crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `src/cli/use_cmd.rs`, `tests/delivery_cross_surface.rs`, `tests/forge_web_project_delivery_contract.rs`) is among them. `cargo fmt --all` was never run. |
| 4.7 `cargo clippy --workspace --all-targets --all-features` | PASS | `cargo clippy --workspace --all-targets --all-features` exited 0 with **39 lines beginning with `warning`** — the same count as the pre-existing baseline. 10 of those are cargo's per-target summary lines and 29 are warning locations, every one of them in a file this change did not touch: `crates/cargo-sonar/src/analyzer.rs` (10), `crates/cargo-audit-boost/src/ui.rs` (3), `crates/cargo-sonar/src/main.rs` (2), `crates/cargo-audit-boost/src/parser.rs` (2), `crates/cargo-audit-boost/src/main.rs` (2), `crates/cargo-audit-boost/src/model.rs` (1), `crates/cargo-audit-boost/src/exporter.rs` (1), `src/registry/model.rs` (1), `src/hosted_ci/store.rs` (1), `src/deploy/model.rs` (1), `src/contract/mod.rs` (1), `src/cli/auth.rs` (1), `src/auth/setup.rs` (1), `tests/origin_repository_retirement_contract.rs` (1), `tests/deploy_ssh_adapter_contract.rs` (1). No warning is in `src/doctor/runner.rs`, `src/doctor/assessment.rs`, `src/doctor/mod.rs`, `src/doctor/claim_evidence_tests.rs`, `src/cli/projects.rs`, `src/mcp/params.rs` or `tests/doctor_claim_evidence_contract.rs`. |
| 4.8 the source file line counts of every touched file | PASS | `wc -l`: `src/doctor/runner.rs` 689, `src/doctor/assessment.rs` 607, `src/doctor/mod.rs` 42, `src/doctor/claim_evidence_tests.rs` 270, `src/doctor/doctor_tests.rs` 987 (unchanged from HEAD), `src/cli/projects.rs` 333, `src/mcp/params.rs` 249, `tests/doctor_claim_evidence_contract.rs` 377, `docs/quickstart.md` 186. Every file is under 1000 physical lines. |
| 5.1 `forge gate --dry-run` | PASS | `./target/debug/forge gate --dry-run --format json` exited 0 and resolved 8 planned required checks (`build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests`), with `unbound required checks: none` and `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. |
| 5.2 `forge gate --timeout-secs 1800` | PASS | `./target/debug/forge gate --timeout-secs 1800 --format json` exited 0 with `aggregate: passed`. All eight required checks are `state: pass` in `.forge/gate/forge/evidence.json`: `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests`. `observed_at: 2026-10-22T06:28:28Z`; `revision: bf1d8034bf02bda19ee2a07e21c167fddcc21bf6` (the parent commit, because the implementation is still uncommitted when the Gate runs); `freshness: fresh`; `profile: full`; `manifest_digest: sha256:2c456f0e82a570fb`; `rule_pack_version: forge-baseline/1.0.0`. The `source-file-size` check reports `evaluated 457 of 457 candidate file(s); maximum 1000 physical line(s)`. The dry run before it resolved the same eight checks with `unbound required checks: none` and `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. |
| 5.3 `openspec validate --all --strict --no-interactive` | PASS | `openspec validate --all --strict --no-interactive` exited 0 with `Totals: 143 passed, 0 failed (143 items)`, including `✓ change/doctor-claim-evidence` and `✓ spec/doctor-maturity-assessment`. |
| 5.4 `node scripts/check-openspec-change-names.mjs` | PASS | `check-openspec-change-names: PASS`. |
| 5.5 `git diff --check` | PASS | `git diff --check` produced no output. |
| the shipped quickstart sample | PASS | `docs/quickstart.md` quoted output the doctor no longer produces. Before: `evidence: manifest runtime agrees with on-disk dependency evidence`. After: `evidence: manifest runtime language 'rust' agrees with profile language 'rust'` plus the second line `evidence: the profile's build definition is present (Cargo.toml)` — the exact wording captured in the 4.3 real run. The sample carries no `registry-observation` block and no `L1-structure` control line (it shows "first findings shown"), so no other line of the sample changed. A repository-wide search over `docs/` and `README.md` for `on-disk dependency evidence`, `profile layout` and `project is not registered` found no other match; the only other occurrences are in `HANDOFF.md` and in the archived change packages, which record history and were not touched. |
