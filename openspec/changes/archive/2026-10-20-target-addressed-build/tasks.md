## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm that no `forge build` command exists and that the profile descriptor already carries a build command for every profile.
- [x] 1.2 Confirm that Forge Core has no operating system, architecture or runtime-identifier vocabulary, and that the only lists in the repository belong to the vendored sibling's distribution layer.
- [x] 1.3 Map the three near-duplicate native-command runners and the one that the build must not copy.
- [x] 1.4 Confirm the `forge test` outcome shape, the error variants and the redaction the Gate uses and the gitops runner does not.
- [x] 1.5 Confirm that the manifest, the profile catalog, the Gate's own build check, the release lane and the deploy lane stay untouched.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add the target vocabulary: the canonical operating systems and architectures, the documented aliases, and the refusal for an unknown value.
- [x] 2.2 Add the per-toolchain resolution: a Rust target triple for `cargo`, a runtime identifier for `dotnet`, a platform and architecture pair for `node` and `npm`, and a refusal for every other toolchain.
- [x] 2.3 Resolve a runtime identifier into the target form, and refuse one for a toolchain that does not read runtime identifiers.
- [x] 2.4 Refuse a partial target and a request that carries two targets.
- [x] 2.5 Add the shared native-command runner: an argument array, environment overrides, a working directory, and redacted evidence.
- [x] 2.6 Add the build operation: it runs the profile's build command with the resolved target bound, and reports the command, the target, the status, the exit code and the evidence.
- [x] 2.7 Add the `forge build` verb next to `forge test`, with the operating system, architecture and runtime-identifier flags, and the two new error variants.
- [x] 2.8 Move the generator's verifier and the readiness runner onto the shared runner.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Check that a build for the host runs the profile's build command and reports the host, and that a failed build command fails the build with its evidence.
- [x] 3.2 Check that a target build binds the toolchain's own environment variable and reports the resolved target in the form the toolchain reads.
- [x] 3.3 Check that a documented alias resolves to the canonical value and that an unknown value is refused by name.
- [x] 3.4 Check that the outcome never reports a target the build did not use.
- [x] 3.5 Check that the generator's verification and the readiness fixtures still behave as before on the shared runner.
- [x] 3.6 Check that `forge test` is unchanged, including its shell semantics.
- [x] 3.7 Check that the Gate's own build check, the manifest contract and the profile catalog are unchanged.
- [x] 3.8 Check that no sibling module is edited.

## 4. Verification

- [x] 4.1 Run the repository test suite: `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1`.
- [x] 4.2 Run the repository Gate: the `forge gate --dry-run` rehearsal and the full `forge gate`.
- [x] 4.3 Run `openspec validate --all --strict --no-interactive` and `git diff --check`.
- [x] 4.4 Build a real project for the host and for a target, and show the reported target in each case.

## 5. Implementation — authorized surfaces (2026-10-20)

- [x] 5.1 `src/target/` (new).
- [x] 5.2 `src/native/` (new).
- [x] 5.3 `src/build/` (new).
- [x] 5.4 `src/generate/engine.rs` and `src/readiness/mod.rs`, whose local runners move onto the shared runner.
- [x] 5.5 `src/cli/gitops.rs` and `src/main.rs`.
- [x] 5.6 `src/core/mod.rs`.
- [x] 5.7 `src/api/command_catalog/rows_project.rs` and `src/api/command_catalog/catalog.rs`. The catalog carries a row for every Clap path, so the new verb needs a row and the catalog's row count moves with it. Found by the repository test suite.
- [x] 5.8 `tests/web_agent_identity_readonly_contract.rs` and `tests/web_release_deploy_history_contract.rs`, whose pinned catalog row count moves with the new row. Found by the repository test suite.
- [x] 5.9 `README.md`, whose core command list gains the new verb.
- [x] 5.10 The new tests.
- [x] 5.11 Record the exact output of every verification command in the evidence table below.
- [x] 5.12 Archive without `--skip-specs` so the capability is promoted into `openspec/specs/target-addressed-build/spec.md`, commit the implementation with its specs, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Repository test suite | PASS | `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` exited 0 and reported 3933 passed, 0 failed. `cargo build` finished with no finding and no warning. Focused runs: `cargo test -p forge --lib build::` 8 passed; `cargo test -p forge --lib target::` 13 passed; `cargo test -p forge --lib native::` 7 passed; `cargo test -p forge --lib generate::` 29 passed; `cargo test -p forge --lib readiness::` 9 passed; `cargo test -p forge --lib gitops::` 7 passed; all 0 failed. Two catalog tests failed at first because the new verb had no command-catalog row and the pinned row counts were stale; the row and the two pinned counts were corrected and the suite re-run. |
| Repository Gate | PASS | `forge gate --dry-run` resolved all 8 required checks with none unbound. The full `forge gate --timeout-secs 1800` run exited 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-20T09:00:59Z"` on revision `a5f74f6`, with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. The `source-file-size` check reported "evaluated 444 of 444 candidate file(s); maximum 1000 physical line(s)". The `security` check reported the pre-existing unmaintained-crate advisories for `paste` and `unsafe-libyaml` and still passed. |
| Strict OpenSpec validation | PASS | `node scripts/check-openspec-change-names.mjs` reports `PASS`; `openspec validate --all --strict --no-interactive` reported `Totals: 129 passed, 0 failed (129 items)`. |
| Whitespace | PASS | `git diff --check` reported no findings. Every changed Rust file is rustfmt-clean under `rustfmt --edition 2021 --check`; `src/main.rs` and `src/lib.rs` still pull in the pre-existing rustfmt-dirty `src/cli/use_cmd.rs`, which was already the case before this change and was left alone. `cargo fmt --all` was not run. |
| Real build | PASS | A `rust-web` project with a minimal Cargo workspace: `forge build` reported `command: cargo build`, `target: host`, `status: passed`, `exit_code: 0`. `forge build --os linux --arch x86_64` reported `target: x86_64-unknown-linux-gnu`, and cargo wrote `target/x86_64-unknown-linux-gnu/`, so the target reached the toolchain. `forge build --os plan9 --arch x86_64` refused with `error[build-invalid]` naming the accepted operating systems. `forge build --os linux` refused with `error[build-invalid]` naming the missing architecture. `forge build --os macos --arch arm64` resolved the aliases to `aarch64-apple-darwin` and the build then failed honestly because that target is not installed on this Linux host. |
