## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm the nine extension surfaces and that no shared contract, state vocabulary or version rule exists between them.
- [x] 1.2 Confirm that every version check in the repository is exact string equality, and that `CONTRACT_MAJOR` in the identity-provider module is never compared.
- [x] 1.3 Map the four places the deploy target kinds are spread over, the two different lists, and the three `ssh` refusal wordings.
- [x] 1.4 Confirm the deploy executor's envelope shape, the exact-string contract check, the self-identification field and the stage attribution format.
- [x] 1.5 Confirm that the manifest contract, the schema version, the plugin registry, the release and delivery lanes and every sibling module stay untouched.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add `src/extension/version.rs`: `ContractVersion` with the family, the major, the minor and the patch.
- [x] 2.2 Add the reader and the one rule: `family/major.minor.patch` is read, the bare form is refused, and compatibility means an equal family and an equal major, with the minor and the patch reported but not gating.
- [x] 2.3 Add `src/extension/state.rs`: `ExtensionState` with `ready`, `disabled`, `unavailable` and `invalid`, every refusal carrying a reason, and `ExtensionDescriptor` with the shared judgement.
- [x] 2.4 Add `src/extension/kinds.rs`: the deploy target kind registry, the wired flag and the one renderer for the supported list.
- [x] 2.5 Add the two refusals in one wording: an unknown kind and a known but unwired kind.
- [x] 2.6 Move the deploy target kind constants into the extension module and re-export them from `src/deploy/contract.rs`, so no caller has to change.
- [x] 2.7 Add `src/deploy/contract.rs`'s parsed executor contract next to the string constant.
- [x] 2.8 Make `src/deploy/engine/model.rs` judge the adapter's declared contract with the shared rule, and read the adapter's self-identification as a family and a version.
- [x] 2.9 Make `src/provider/runner.rs` judge the same envelope's declared contract with the same shared rule, so the deploy lane and the provider probe cannot disagree.
- [x] 2.10 Add `src/extension/contract.rs` with the shared verification, and `src/extension/extension_tests.rs` with the suite that runs it over every registered kind.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Make the configuration validator refuse an unknown and an unwired kind through the registry, in the shared wording.
- [x] 3.2 Make the plan and the observe path refuse an unwired kind through the registry, in the same wording and under one error code.
- [x] 3.3 Check that an adapter at a newer minor of the same family is accepted where it was refused before, by the deploy lane and by the provider probe.
- [x] 3.4 Check that a different major and a different family are refused with the reason spelled out, and that an unreadable version is refused by value.
- [x] 3.5 Check that the stage evidence records the adapter's family and version, and that a malformed self-identification is refused while an absent one is still allowed.
- [x] 3.6 Check that every kind in the registry is accepted or refused consistently by the configuration validator and by the plan.
- [x] 3.7 Check that the deploy executor's wire format, its argv, its timeout and its persisted state are unchanged.
- [x] 3.8 Check that the manifest contract, the schema version, the plugin registry, the release and delivery lanes and every sibling module are unchanged.
- [x] 3.9 Check that the adapter contract document states the rule the code now applies.

## 4. Verification

- [x] 4.1 Run the repository test suite: `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1`.
- [x] 4.2 Run the repository Gate: the `forge gate --dry-run` rehearsal and the full `forge gate`.
- [x] 4.3 Run `openspec validate --all --strict --no-interactive` and `git diff --check`.
- [x] 4.4 Run a real deploy against a fixture adapter that declares a newer minor, one that declares a different major, and one that names itself, and show each outcome.

## 5. Implementation — authorized surfaces (2026-10-20)

- [x] 5.1 `src/extension/` (new): `mod.rs`, `version.rs`, `state.rs`, `kinds.rs`, `contract.rs`, `extension_tests.rs`.
- [x] 5.2 `src/lib.rs`, which gains the module registration.
- [x] 5.3 `src/deploy/contract.rs`, which re-exports the kind constants and gains the parsed executor contract.
- [x] 5.4 `src/deploy/prepare.rs`, `src/deploy/model.rs` and `src/deploy/engine/stages.rs`, whose kind validation and refusals read the registry.
- [x] 5.5 `src/deploy/engine/model.rs`, whose contract check uses the shared version rule.
- [x] 5.6 `src/provider/runner.rs`, whose deploy-envelope contract check uses the same shared version rule, and `src/provider/matrix.rs`, whose boundary description states it.
- [x] 5.7 `docs/adapter-contracts/deploy-executor.md`, which states the rule the code now applies.
- [x] 5.8 The new and updated tests.
- [x] 5.9 Record the exact output of every verification command in the evidence table below.
- [x] 5.10 Archive without `--skip-specs` so the capability is promoted into `openspec/specs/extension-point-contract/spec.md`, commit the implementation with its specs, then update the handoff pointer. — Archived at `openspec/changes/archive/2026-10-20-extension-point-contract/` without `--skip-specs`; five requirements promoted into `openspec/specs/extension-point-contract/spec.md` and the archive's `TBD` Purpose replaced with the real one before the commit. The implementation and its specs are one commit; the handoff pointer update is the second.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Repository test suite | PASS | `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` → exit 0; 203 test binaries; **3947 passed, 0 failed, 1 ignored**. The Forge lib binary alone: `test result: ok. 1339 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out`. |
| Focused suites | PASS | `cargo test -p forge --lib extension::` → 9 passed, 0 failed. `cargo test -p forge --lib deploy::` → 42 passed, 0 failed. `cargo test -p forge --test deploy_contract` → 20 passed, 0 failed. `cargo test -p forge --test deploy_adapter_contract` → 10 passed, 0 failed. `cargo test -p forge --test deploy_cross_surface` → 5 passed, 0 failed. `cargo test -p forge --lib provider::` → 28 passed, 0 failed. |
| Build | PASS | `cargo build` → `Finished dev profile ...`, no warnings. `cargo clippy --workspace --all-targets --all-features` reports no finding in any file this change creates or edits (the pre-existing findings in `crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `src/auth/setup.rs`, `src/cli/auth.rs`, `src/contract/mod.rs`, `src/hosted_ci/store.rs`, `src/registry/model.rs` and `tests/origin_repository_retirement_contract.rs` are untouched and outside this change). |
| Repository Gate | PASS | `forge gate --dry-run` resolved all 8 required checks with none unbound; the exemption budget reported `cap=2 declared=0 headroom=2 required=8 verdict=within`. The full `forge gate --timeout-secs 1800` run exited 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-20T11:35:04Z"` on revision `c3755c2`, with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. The `source-file-size` check reported "evaluated 450 of 450 candidate file(s); maximum 1000 physical line(s)". The `security` check reported the pre-existing unmaintained-crate advisories for `paste` and `unsafe-libyaml` and still passed. |
| Strict OpenSpec validation | PASS | `openspec validate --all --strict --no-interactive` → `✓ change/extension-point-contract`; `Totals: 130 passed, 0 failed (130 items)`; exit 0. `node scripts/check-openspec-change-names.mjs` → `check-openspec-change-names: PASS`; exit 0. |
| Whitespace | PASS | `git diff --check` → no output, exit 0. The new `src/extension/*` files were checked the same way (`git diff --no-index --check /dev/null <file>`): no output. Every created or changed file is rustfmt-clean under `rustfmt --edition 2021 --check <file>`; `cargo fmt --all` was not run. |
| Real deploy | PASS | A `local`-target project with a committed `forge.yaml` and a fixture adapter on `FORGE_DEPLOYER_BIN`. The adapter declaring `forge-deploy-executor/0.2.0` with `source: forge-deployer-jenkins/0.1.0` was **accepted**: `forge deploy apply --target-name local --confirm --dry-run` exited 0 with `summary: deploy applied to target `local` and observed as `running`` and the stage evidence `executor=forge-deployer-jenkins/0.1.0@cafe1234`, so the family and the version are recorded. The adapter declaring `forge-deploy-executor/1.0.0` was refused with `error[deploy-target-unavailable]`: "forge-deployer-jenkins declares the contract `forge-deploy-executor/1.0.0`, which is incompatible with the required `forge-deploy-executor/0.1.0`: the major versions differ (1 and 0)". The adapter declaring `forge-deployer/0.1.0` was refused with "the families differ (`forge-deployer` and `forge-deploy-executor`)". The bare `0.1.0` was refused with "deploy adapter contract `0.1.0` cannot be read: the version `0.1.0` does not name a contract family; expected `family/major.minor.patch`". The adapter whose `source` was the bare `0.1.0` was refused with "deploy adapter self-identification `0.1.0` cannot be read: the version `0.1.0` does not name a contract family; expected `family/major.minor.patch`". Every refusal exited 1. A project whose target kind is `ssh` was refused by `forge deploy plan` and by `forge deploy apply` with the same words and the same code: `error[deploy-invalid]: deploy invalid: deployment target kind `ssh` is known but not yet wired; v0.1.0 supports `local`, `docker-compose`, `jenkins`, and `mac-runtime``. A project whose target kind is `carrier-pigeon` was refused with `error[deploy-invalid]`: "deployment target kind `carrier-pigeon` is not supported; expected one of `local`, `docker-compose`, `jenkins`, and `mac-runtime`". |
| Behaviour change: the `ssh` refusal code | RECORDED | The plan and the observe path now report `deploy-invalid` instead of `deploy-target-unavailable`, in the registry's one wording: `deployment target kind `ssh` is known but not yet wired; v0.1.0 supports `local`, `docker-compose`, `jenkins`, and `mac-runtime``. The configuration validator refuses `ssh` with the same words and the same code. Covered by `src/deploy/engine/stages_tests.rs::ssh_target_is_refused_by_the_registry_in_one_wording`, `::observe_refuses_an_unwired_kind_in_the_same_words_as_the_plan`, `::every_registered_kind_is_refused_consistently_by_the_configuration_and_the_plan` and `tests/deploy_contract.rs::plan_refuses_ssh_target_with_invalid_boundary`. |
| Behaviour change: a malformed `source` | RECORDED | An envelope whose `source` is present but unreadable is refused with `deploy-target-unavailable`, naming the value (`deploy adapter self-identification `0.1.0` cannot be read: the version `0.1.0` does not name a contract family; expected `family/major.minor.patch``). An absent `source` is still allowed and attributes to the configured binary with an `unknown` revision. Covered by `src/deploy/engine/stages_tests.rs::a_malformed_self_identification_is_refused_and_an_absent_one_is_allowed`. |
| Behaviour change: the adapter's declared contract | RECORDED | A newer minor of the same family is accepted (`forge-deploy-executor/0.2.0` where `0.1.0` is required) by the deploy lane and by the provider probe; a different major or family is refused naming both; an unreadable value is refused by value. Covered by `src/deploy/engine/stages_tests.rs::executor_envelope_accepts_a_newer_minor_of_the_same_family`, `::executor_envelope_refuses_a_different_major_or_family`, `::executor_envelope_requires_namespaced_contract` and `src/provider/provider_tests.rs::deploy_fixture_partial_and_failed_states`. |
| Reading: the bare `major.minor.patch` form | RESOLVED | `proposal.md` first said the shared reader reads "`family/major.minor.patch` (and the bare `major.minor.patch`)", while `design.md` decides that the bare form "carries no family and is therefore unreadable as a contract". The decisive design governs: the bare `0.1.0` is refused and the refusal names the value and the expected shape. The proposal was corrected before the archive so the package states one rule. |
