# Tasks: release-gate-mandatory

## 1. Plan

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the release lane's check model: `src/release/engine/prepare.rs::collect_captured_checks`, `run_one_check`, `run_gate_check` and `check_readiness`.
- [x] 1.3 Read the shared rule: `src/gate/ship_gate.rs::evaluate_ship_gate` and its unit tests.
- [x] 1.4 Read the manifest vocabulary: `src/release/model.rs::ReleaseConfig::from_manifest_meta` and the `release.checks` model.
- [x] 1.5 Read the apply step: `src/release/engine/apply.rs::apply_release` and the order of the readiness judgement against the `--confirm` guard.
- [x] 1.6 Read the canonical capabilities this change modifies: `release-evidence-non-vacuous` and `release-publishing`.
- [x] 1.7 Inventory every release fixture that would need Gate evidence.

## 2. Design

- [x] 2.1 Decide that the Gate check always applies to a release, and that `release.checks` chooses the extra checks only.
- [x] 2.2 Decide that `kind: gate` is refused rather than ignored.
- [x] 2.3 Decide that the release lane calls the shared rule and reports its verdict unchanged.
- [x] 2.4 Decide that the evidence is compared with the manifest's project id.
- [x] 2.5 Decide that the `--confirm` guard keeps its place after the readiness judgement.

## 3. Implement

- [x] 3.1 `src/release/model.rs`: `kind: gate` is refused, with the reason that names the truth.
- [x] 3.2 `src/release/engine/prepare.rs`: the Gate check always applies and calls the shared rule; the private copy is deleted.
- [x] 3.3 The apply step re-checks the Gate evidence at the applied revision.
- [x] 3.4 Every release fixture writes a Gate evidence record bound to its own revision.
- [x] 3.5 The tests pin the new behaviour: the Gate check always applies, a `kind: gate` manifest is refused, and the release refuses for the same reason the delivery lane does.

## 4. Verify

- [x] 4.1 A real `forge release prepare` on a project without Gate evidence refuses and names the Gate.
- [x] 4.2 A real `forge release prepare` on a project with fresh, full-profile Gate evidence is ready.
- [x] 4.3 A real `forge release apply` on a project whose Gate evidence is stale for the applied revision refuses before any stage.
- [x] 4.4 The repository test suite passes.
- [x] 4.5 The repository Gate passes, locally, before the archive.
- [x] 4.6 `openspec validate --all --strict --no-interactive`, `node scripts/check-openspec-change-names.mjs` and `git diff --check` pass.
- [x] 4.7 Every changed Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.

## 5. Archive

- [x] 5.1 Archive without `--skip-specs`, so the new capability is promoted.
- [x] 5.2 Replace the archive's `TBD` Purpose with the real one.
- [x] 5.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 6. Evidence

The implementation is in place, the full repository test suite passes, and the
Gate has passed locally, before the archive. The archive (5.x) is the owner's
remaining step.

| Check | Result | Evidence |
|---|---|---|
| The change package validates | PASS | `openspec validate --all --strict --no-interactive` → `✓ change/release-gate-mandatory`, `Totals: 136 passed, 0 failed (136 items)`. `node scripts/check-openspec-change-names.mjs` → `check-openspec-change-names: PASS`. |
| The implementation | PASS | `cargo build` → `Finished \`dev\` profile`. `src/release/model.rs` refuses `kind: gate`; `src/release/engine/prepare.rs` captures the Gate check from `crate::gate::ship_gate::evaluate_ship_gate(project_dir, &manifest.project.id, revision)` and reports the verdict unchanged; the private `run_gate_check` copy is deleted. `src/release/engine/apply.rs` is unchanged: the readiness judgement still runs before the `--confirm` guard. |
| The repository test suite | PASS, 4019 tests / 0 failed | The owner's run: `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` exited 0 over 208 test binaries: 4019 passed, 0 failed. The 4010-test baseline plus the 9 new tests. The implementer's focused runs: `cargo test -p forge --lib release::` → `34 passed; 0 failed`; `cargo test --test release_contract` → `10 passed; 0 failed`; `cargo test --test release_cross_surface` → `4 passed; 0 failed`; `cargo test --test forge_web_project_release_contract` → `8 passed; 0 failed`; `cargo test --test release_gate_mandatory_contract` → `5 passed; 0 failed`; `cargo test --test gate_cross_surface` → `13 passed; 0 failed`; `cargo test --test project_changelog_bootstrap_contract` → `4 passed; 0 failed`. |
| The repository Gate | PASS, 8/8 | `forge gate --dry-run` resolved all 8 required checks with none unbound. The full `forge gate --timeout-secs 1800` run exited 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-21T02:28:52Z"` on revision `dafc069`, with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. `source-file-size` evaluated 456 of 456 candidates with a maximum of 1000 physical lines. Two earlier runs were blocked by the machine, not by this change: the `declared-verification` check and then the `tests` check each exceeded their 600000ms deadline while an unrelated workspace on the same machine was compiling (load average 24.6 on 12 cores). The third run, with the load back to 4.8, passed every check. The failing commands were the repository's own suite, which passes standalone. |
| Real runs | PASS | A scratch project with a `release:` block declaring `doctor` and `test` and no Gate evidence: `forge release prepare` reported `ready: no` with the row `gate fail applicable=true revision=d46eee51e381: Forge Gate evidence is absent for \`demo-app\`; run \`forge gate --profile full\` from the project directory and retry` and the summary `release plan not ready: failing check(s) gate:fail; release execution blocks before tag or publication`. With a Gate evidence record bound to the project's HEAD (`contract 0.2.0`, `profile full`, `aggregate passed`, `dry_run false`): `ready: yes`, `gate pass applicable=true`, `release plan ready for \`demo-app\`: 3 check(s) passed at revision \`d46eee51e381\``. After a new commit, `forge release apply --version 1.0.0 --confirm` refused with `error[release-check-failed]: release check failed: failing check(s) gate:fail at revision \`9e617a193712\`; release execution blocks before tag or publication`, left no tag and did not move HEAD. A manifest declaring `kind: gate` was refused with `error[release-invalid]: release invalid: release.checks kind \`gate\` is not configurable; the Gate check always applies to a release`. No `forge gate` run was made: the evidence record for the second run was written by hand, bound to `git rev-parse HEAD`. |
| The two lanes refuse for the same reason | PASS | The owner's run on one project at revision `aca7a0109574`: `forge delivery preflight rel-gate-demo` refused with `error[delivery-conflict]: delivery conflict: Forge Gate evidence for \`rel-gate-demo\` must use a supported contract, the full profile, and be fresh and passed for revision aca7a0109574; observed contract=0.2.0, profile=full, project=rel-gate-demo, aggregate=passed, freshness=stale; run \`forge gate --profile full\` from the project directory and retry`, and `forge release prepare` on the same tree reported the identical sentence in its Gate check row. The rule's reason reaches both lanes unchanged. |
| OpenSpec, names, whitespace | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 136 passed, 0 failed (136 items)`. `node scripts/check-openspec-change-names.mjs` → `check-openspec-change-names: PASS`. `git diff --check` → clean. |
| Lint and formatting | PASS | `rustfmt --edition 2021 --check` is clean on every touched file (`src/release/model.rs`, `src/release/engine/prepare.rs`, `src/release/prepare.rs`, `src/release/engine/engine_tests.rs`, `tests/release_contract.rs`, `tests/release_cross_surface.rs`, `tests/forge_web_project_release_contract.rs`, `tests/release_gate_mandatory_contract.rs`, `tests/gate_cross_surface.rs`). `cargo clippy --workspace --all-targets --all-features` reports no warning in any touched file (the five remaining lib warnings are pre-existing in `src/auth/setup.rs`, `src/contract/mod.rs`, `src/deploy/model.rs`, `src/hosted_ci/store.rs` and `src/registry/model.rs`). `cargo fmt --all` was not run; no pre-existing rustfmt-dirty file was touched. |
