# Tasks: profile-toolchain-version-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the profile catalogue: `src/profile/catalog.rs` (the six supported descriptors, their `toolchain` and `toolchain_version`).
- [x] 1.3 Read the preflight: `src/profile/resolve.rs` (`preflight_profile`, `toolchain_on_path`) and the `PreflightReport` model.
- [x] 1.4 Read the readiness matrix: `src/readiness/mod.rs` (`run_profile_row`, `capture_toolchain_version`, `unverified_row`, `run_native_command`).
- [x] 1.5 Read the kit registry's ecosystem map: `src/kit/registry.rs` (`Ecosystem::for_toolchain`).
- [x] 1.6 Read the toolchain error vocabulary: `src/core/mod.rs` (`ToolchainMissing` and its code) and the generator's `--verify-native` path.
- [x] 1.7 Read the profile inspection output: `src/cli/feature.rs` (the `toolchain:` line).
- [x] 1.8 Read the qualification evidence: `docs/release-readiness.md` and `.github/workflows/ci.yml` (the provisioned toolchain versions).
- [x] 1.9 Read the canonical capabilities this change modifies: `profile-registry`, `profile-and-release-readiness`.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the declared version is a requirement on the runtime's version, compared component-wise on the declared version's component count.
- [x] 2.2 Decide the runtime: the ecosystem's binary, so the Node ecosystem's runtime is `node` and both Node-ecosystem profiles check the same node version.
- [x] 2.3 Decide the catalogue correction: `nextjs-web` and `react-web` declare the node version the repository's own CI qualification provisions; the other three and the one absent declaration are unchanged.
- [x] 2.4 Decide the preflight's verdict: a version mismatch is its own typed error, never `toolchain-missing`.
- [x] 2.5 Decide the matrix's verdict: a version mismatch is `unverified` (never passing), the native commands are not run, and the row records the declaration, the observation and the verdict.
- [x] 2.6 Decide the guard: the rule, the catalogue, the preflight and the matrix are pinned by a contract check.

## 3. Implement

- [x] 3.1 `src/profile/version.rs` (new): the banner extraction, the component-wise comparison, the runtime resolution and the version probe, with the rule's own unit tests.
- [x] 3.2 `src/profile/resolve.rs`: the preflight checks the version and its report carries the observed version.
- [x] 3.3 `src/core/mod.rs`: the `ToolchainVersionMismatch` error and its `toolchain-version-mismatch` code.
- [x] 3.4 `src/profile/catalog.rs`: `nextjs-web` and `react-web` declare the version the repository's qualification provisions.
- [x] 3.5 `src/readiness/mod.rs`: the matrix checks the version, records the declaration, the runtime and the verdict, and the existing version capture becomes the shared probe.
- [x] 3.6 `src/cli/feature.rs`: the toolchain line names the runtime it declares.
- [x] 3.7 `docs/release-readiness.md`: the qualification basis and the row states.
- [x] 3.8 `tests/profile_toolchain_version_truth_contract.rs` (new): the pinned rule, catalogue, preflight and matrix.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 `forge profile preflight nextjs-web` on a host whose node version does not match: refused with `toolchain-version-mismatch`, naming both versions.
- [x] 4.4 `forge profile preflight rust-web`: unchanged, because the profile declares no version.
- [x] 4.5 `forge readiness matrix`: the row for a mismatched host is `unverified` with the mismatch named, and the row records the declaration next to the observation.
- [x] 4.6 `forge profile inspect react-web`: the toolchain line names the runtime it declares.
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
- [x] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capabilities' purposes.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

Closed by the owning agent: the change was archived with `openspec archive
profile-toolchain-version-truth --yes` (no `--skip-specs`), so
`openspec/specs/profile-toolchain-version-truth/spec.md` was created with
four requirements, one requirement of `profile-registry` was modified with
every canonical scenario kept and one scenario added, and one requirement of
`profile-and-release-readiness` was modified with every canonical scenario
kept and one scenario added. The archive's `TBD` Purpose was replaced by hand
with the paragraph the capability's own requirements describe, and both
modified capabilities' Purposes now state that a declared toolchain version
is a requirement the surfaces check. The implementation and its specs are
committed together, and the handoff update is a separate commit.

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed.

| Check | Result | Evidence |
|---|---|---|
| 4.1 `cargo build --bin forge` | PASS | `cargo build --workspace` exited 0; `cargo test --workspace --no-run` exited 0 and produced every test binary. The binary is at `/home/paul/code/forge/target/debug/forge`. |
| 4.2 the repository test suite | PASS, 4123 tests / 0 failed | `bash scripts/check-tests.sh` — the repository Gate's own `tests` command — exited 0. The `test result:` lines sum to **4123 passed, 0 failed, 1 ignored** across 221 test binaries, and the four python checks (`test_dotnet_platform_libraries_import.py`, `check-dotnet-platform-libs-import.py`, `test_rust_platform_libraries_import.py`, `check-rust-platform-libs-import.py`) each printed `ok`. The identity-providers workspace suite also ran. The 4083-test baseline plus the new contract test's 12 cases and the version rule's 28 unit cases. |
| 3.8 `cargo test --test profile_toolchain_version_truth_contract` | PASS | `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s`. The twelve cases: `every_supported_profile_declares_the_qualified_version`, `the_ci_workflow_provisions_the_qualified_node_version`, `every_declared_version_is_a_version_its_runtime_can_have`, `the_rule_judges_the_declared_components`, `the_banner_yields_the_version`, `the_node_ecosystem_runs_on_node`, `a_host_of_the_wrong_version_is_refused_with_the_mismatch_code`, `a_host_of_the_declared_version_passes_and_names_what_it_checked`, `a_profile_without_a_declaration_is_judged_on_presence_only`, `a_row_on_a_mismatched_version_is_unverified_and_runs_no_native_command`, `a_row_of_the_npm_ecosystem_names_the_runtime_it_probed`, `a_row_without_a_declaration_records_no_requirement`. |
| 3.1 `cargo test -p forge --lib profile::` | PASS | `test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 1372 filtered out`. The rule's own cases are in `profile::version::tests` and the preflight's in `profile::resolve::tests`. |
| 4.3 `forge profile preflight` on a mismatched host | PASS | **Before**: every profile reported `preflight ok: toolchain '<name>' available for profile '<id>' (version 0.1.0)` — the check was the binary's presence on `PATH`, never its version. **After**, on this host (`python3` 3.14.4 against the declared 3.12): `forge profile preflight python-service` → `error[toolchain-version-mismatch]: toolchain 'python3' required by profile 'python-service' is version 3.14.4; the profile declares 3.12; the profile was not tested`. The same for `flutter-app`: `error[toolchain-version-mismatch]: toolchain 'flutter' required by profile 'flutter-app' is version 3.47.1; the profile declares 3.22; the profile was not tested`. On the matching host: `forge profile preflight nextjs-web` → `preflight ok: toolchain 'node' version 24.18.0 satisfies profile 'nextjs-web' (node@24)`, and `forge profile preflight react-web` → `preflight ok: toolchain 'npm' (node) version 24.18.0 satisfies profile 'react-web' (node@24)`. |
| 4.4 `forge profile preflight rust-web` | PASS | `preflight ok: toolchain 'cargo' available for profile 'rust-web' (version 0.1.0)` — unchanged. `rust-web` declares `stable`, a moving channel rather than a version, so it states no version requirement and is judged on presence only. `forge profile preflight aspnet-web` → `preflight ok: toolchain 'dotnet' version 10.0.400 satisfies profile 'aspnet-web' (dotnet@10.0)`. |
| 4.5 `forge readiness matrix` | PASS | `forge readiness matrix` exited 0: `readiness matrix (contract 0.1.0): passed=4 failed=0 unverified=2 ready=false`. `aspnet-web` passed (`dotnet 10.0.400` against the declared `10.0`), `nextjs-web` passed (`node 24.18.0` against `24`), `react-web` passed (`toolchain=npm runtime=node version=24.18.0 declared=24`), `rust-web` passed (`cargo 1.99.1`, no declaration), and the two rows whose host versions are newer than the declaration are `unverified` with the mismatch named: `toolchain 'python3' for profile 'python-service' is version 3.14.4; the profile declares python3@3.12; the profile was not tested` and the same for `flutter-app` (`3.47.1` against `3.22`). The unverified rows ran no native command. The qualified gate `forge readiness check --profile rust-web --profile nextjs-web --profile aspnet-web` — the one CI runs — exited 0 with `gate ready=true` over `passed=3 failed=0 unverified=0 ready=true`. |
| 4.6 `forge profile inspect react-web` | PASS | **Before**: `toolchain: npm@20` — a version npm does not have. **After**: `toolchain: npm (node@24)`; `forge profile inspect nextjs-web` → `toolchain: node@24`; `forge profile inspect rust-web` → `toolchain: cargo` (a channel states no version). |
| 4.7 `rustfmt --edition 2021 --check` on every touched Rust file | PASS | `rustfmt --edition 2021 --check` reported no diff for `src/profile/version.rs`, `src/profile/resolve.rs`, `src/profile/catalog.rs`, `src/profile/model.rs`, `src/profile/mod.rs`, `src/readiness/mod.rs`, `src/cli/feature.rs`, `src/cli/setup.rs`, `src/core/mod.rs`, `tests/profile_toolchain_version_truth_contract.rs` and `tests/readiness_contract.rs`. `git status --porcelain` shows none of the 12 pre-existing rustfmt-dirty files (`crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `src/cli/use_cmd.rs`, `tests/delivery_cross_surface.rs`, `tests/forge_web_project_delivery_contract.rs`) among the modified paths. `cargo fmt --all` was never run. |
| 4.8 `cargo clippy --workspace --all-targets --all-features` | PASS | exit code 0 with **39 lines beginning with `warning`** — the same count as the pre-existing baseline. No warning points into a touched file. |
| 4.9 the source file line counts of every touched file | PASS | `src/profile/version.rs` 390, `src/profile/resolve.rs` 720, `src/profile/catalog.rs` 662, `src/profile/model.rs` 128, `src/profile/mod.rs` 29, `src/readiness/mod.rs` 971, `src/cli/feature.rs` 833, `src/cli/setup.rs` 541, `src/core/mod.rs` 694, `tests/profile_toolchain_version_truth_contract.rs` 261, `tests/readiness_contract.rs` 268, `docs/release-readiness.md` 124 — every file under 1000 physical lines. |
| 5.1 `forge gate --dry-run` | PASS | exit 0; `checks (8 planned):` — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests`; `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`; `unbound required checks: none`. |
| 5.2 `forge gate --timeout-secs 1800` | PASS | `./target/debug/forge gate --timeout-secs 1800 --format json` exited 0 with `aggregate: passed`. All eight required checks are `state: pass` in `.forge/gate/forge/evidence.json`: `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests`. `observed_at: 2026-10-22T09:49:05Z`; `revision: 049698661762abd7b89e846e2003fb019896bdad` (the parent commit, because the implementation is still uncommitted when the Gate runs); `freshness: fresh`; `profile: full`; `manifest_digest: sha256:2c456f0e82a570fb`; `rule_pack_version: forge-baseline/1.0.0`. The tree was warmed first (`cargo build --workspace`, `cargo test --workspace --no-run`) and nothing else was running on the machine, so the `build` check did not lose the shared build lock. |
| 5.3 `openspec validate --all --strict --no-interactive` | PASS | `Totals: 144 passed, 0 failed (144 items)`, including `✓ change/profile-toolchain-version-truth`. |
| 5.4 `node scripts/check-openspec-change-names.mjs` | PASS | `check-openspec-change-names: PASS` (exit 0) |
| 5.5 `git diff --check` | PASS | no output, exit 0 |
