# Tasks: lifecycle-observation-closure

## 1. Plan

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`, `.ai-rules/architecture.md`.
- [x] 1.2 Map the doctor surface: `src/doctor/` (`model.rs`, `runner.rs`, `probes.rs`, `assessment.rs`, `freshness.rs`, `gaps/`), its finding IDs, its report shape and its CLI entry.
- [x] 1.3 Map every caller of `run_doctor`: `src/cli/projects.rs` (the doctor CLI and the external checker), `src/api/handlers_changes.rs`, `src/api/status.rs`, `src/api/workbench.rs`, `src/mcp/params.rs` and `src/release/engine/prepare.rs`.
- [x] 1.4 Map the delivery state: the `operations` journal, the `DeliveryVerb` and `DeliveryPhase` vocabularies, the `build_delivery_report` projection and the `forge delivery status` CLI.
- [x] 1.5 Confirm that the delivery projection is read-only and that no other surface computes a delivery state.
- [x] 1.6 Confirm that the doctor's canonical capability and its finding inventory are the only specs that bound this change.

## 2. Design

- [x] 2.1 Decide where the delivery state enters the doctor: the caller-supplied `RegistryObservation`, never a second registry handle inside the doctor.
- [x] 2.2 Decide the observation's shape: `Option<DeliveryObservation>` on the outer field ("the caller did not look") and a three-state enum inside ("recorded", "absent", "unreadable").
- [x] 2.3 Decide the phase-to-status rule: `PASS` for healthy, `FAIL` for a failed delivery, `WARN` for a delivery in progress and for a failed optional enrolment, not applicable for no delivery, `UNAVAILABLE` for a state that cannot be read.
- [x] 2.4 Decide the finding id and its evidence: `delivery-state`, naming the phase, the environment, the revision and the time the state last changed.
- [x] 2.5 Confirm the read-only guarantee: the doctor selects from the journal and writes nothing, and never runs a delivery verb.

## 3. Implement

- [x] 3.1 `src/doctor/model.rs`: add `DeliveryObservation` and the `delivery` field on `RegistryObservation`.
- [x] 3.2 `src/doctor/observation.rs` (new): `delivery_observation`, the one function that reads the delivery state through the delivery lane's door.
- [x] 3.3 `src/doctor/delivery.rs` (new): the probe and the phase-to-status rule.
- [x] 3.4 `src/doctor/runner.rs`: call the probe and push the finding.
- [x] 3.5 `src/cli/projects.rs`: fill the observation in the doctor CLI and in the external checker.
- [x] 3.6 `src/api/handlers_changes.rs`, `src/api/status.rs`, `src/api/workbench.rs` and `src/mcp/params.rs`: fill the observation with the same function. (The workbench and the MCP transport also report a doctor report; the design named only the first three, and every surface that reports one reads the same state.)
- [x] 3.7 `src/release/engine/prepare.rs`: pass no observation, as today.
- [x] 3.8 Update every construction site of `RegistryObservation`, including the doctor's own tests.
- [x] 3.9 Keep every source file under the repository's 1000-line cap.

## 4. Verify

- [x] 4.1 The doctor's own suite covers the rule: healthy, failed, in progress, absent, unreadable and not registered.
- [x] 4.2 An integration test drives `forge doctor` against a real registry with real delivery rows, and shows the finding and the verdict.
- [x] 4.3 The integration test shows the doctor writes nothing: the journal and the registry bytes are unchanged.
- [x] 4.4 The repository test suite passes.
- [x] 4.5 The repository Gate passes, locally, before the archive. (The first run failed on a build timeout and two advisories published that day; the dependency bump above and a warm build fixed both. The second run passed 8/8.)
- [x] 4.6 `openspec validate --all --strict --no-interactive`, `node scripts/check-openspec-change-names.mjs` and `git diff --check` pass.

## 5. Archive

- [x] 5.1 Archive without `--skip-specs`, so the new capability is promoted and `doctor-maturity-assessment` is replaced.
- [x] 5.2 Replace the archive's `TBD` Purpose with the real one.
- [x] 5.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 6. Verification evidence

Recorded by the implementing agent on 2026-10-21, before the archive.

| Command | Outcome |
|---|---|
| `cargo build` | `Finished dev profile [unoptimized + debuginfo] target(s)` |
| `cargo test -p forge --lib doctor` | `test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 1327 filtered out` |
| `cargo test --test doctor_delivery_observation_contract` | `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` | exit code 0, no failing test binary. The main agent re-ran it: **4002 passed, 0 failed, 1 ignored**, and `grep -c "^failures:"` reported 0. |
| `cargo clippy --workspace --all-targets --all-features` | no warning in any file this change touched; the remaining warnings are pre-existing (`crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `src/auth/setup.rs`, `src/cli/auth.rs`, `src/contract/mod.rs`, `src/deploy/model.rs`, `src/hosted_ci/store.rs`, `src/registry/model.rs`, `tests/deploy_ssh_adapter_contract.rs`, `tests/origin_repository_retirement_contract.rs`) |
| `rustfmt --edition 2021 --check` on every touched file | clean (`src/doctor/delivery.rs`, `src/doctor/delivery_tests.rs`, `src/doctor/doctor_tests.rs`, `src/doctor/model.rs`, `src/doctor/mod.rs`, `src/doctor/observation.rs`, `src/doctor/runner.rs`, `src/cli/projects.rs`, `src/api/handlers_changes.rs`, `src/api/status.rs`, `src/api/workbench.rs`, `src/mcp/params.rs`, `tests/doctor_delivery_observation_contract.rs`) |
| `openspec validate --all --strict --no-interactive` | `Totals: 133 passed, 0 failed (133 items)` |
| `node scripts/check-openspec-change-names.mjs` | `check-openspec-change-names: PASS` |
| `git diff --check` | clean |
| Repository Gate — first run | FAILED for two reasons, neither caused by this change. The `build` check timed out (`adapter 'project-runtime' exceeded its timeout of 60000ms`) because the dependency graph had just changed and `cargo build` was still compiling; the `security` check failed on two advisories published on 2026-10-19 and 2026-10-20 — `rustls-webpki` 0.103.15 (RUSTSEC-2026-0382) and `webpki-roots` 1.0.9 (RUSTSEC-2026-0381). Both crates are transitive and unchanged by this change. |
| Dependency bump (main agent) | `cargo update -p rustls-webpki --precise 0.103.16` and `cargo update -p webpki-roots --precise 1.0.10` — the two patch upgrades the advisories name. `cargo audit` then exited 0, with only the pre-existing warnings for `paste`, `unsafe-libyaml` and `lru` left. `Cargo.lock` changed by four lines; no source file changed. |
| Repository Gate — second run | PASS. `forge gate --dry-run` resolved all 8 required checks with none unbound. The full `forge gate --timeout-secs 1800` run exited 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-20T20:19:46Z"` on revision `561d156`, with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. The `source-file-size` check reported "evaluated 456 of 456 candidate file(s); maximum 1000 physical line(s)". The `security` check reported the pre-existing unmaintained-crate advisories for `paste`, `unsafe-libyaml` and `lru` and still passed. |
| A real doctor run | A fixture project at `/tmp/forge-change11/project` registered in a real registry, with a real `delivery.stage` journal row that failed. `forge --registry … doctor …` reported `[FAIL] delivery-state (manual)` with the evidence `delivery phase=stage-failed`, `environment=stage`, `delivered revision=0123456789abcdef0123456789abcdef01234567`, `delivery state last changed 2026-10-20T19:31:00Z`, and the verdict `not healthy`. A second fixture with no delivery row reported `[PASS] delivery-state (automatic)` with `applicability: not applicable`. The registry's md5 was identical before and after the doctor run and the journal row count did not change: the doctor read the delivery and wrote nothing. |
