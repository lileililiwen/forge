# Tasks: one-native-command-runner

## 1. Plan

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the shared native runner: `src/native/mod.rs` and its unit tests.
- [x] 1.3 Read the process primitive: `src/process.rs::spawn_with_timeout` and `src/studio/preview.rs`'s group-kill pattern.
- [x] 1.4 Read the test operation: `src/gitops/mod.rs::run_test` and `run_shell_command`.
- [x] 1.5 Read the callers of the shared runner: `src/build/mod.rs`, `src/generate/engine.rs::verify_native`, `src/readiness/mod.rs`.
- [x] 1.6 Read the canonical capabilities this change modifies: `agent-runtime-workflows` and `profile-and-release-readiness`.

## 2. Design

- [x] 2.1 Decide that `src/native/mod.rs` is the one native command runner, and that it gains a shell entry point.
- [x] 2.2 Decide the bound: 1800 seconds, not configurable.
- [x] 2.3 Decide the kill: a process-group kill, using the pattern `src/studio/preview.rs` already established.
- [x] 2.4 Decide that the process-group lifecycle lives in `src/process.rs`.
- [x] 2.5 Decide that the adapter runners are out of scope, and record their remediation path.

## 3. Implement

- [x] 3.1 `src/process.rs`: `spawn_in_own_group` and `kill_process_tree`, used by `spawn_with_timeout`.
- [x] 3.2 `src/native/mod.rs`: the bound, the group spawn, the group kill, `run_native_shell`, and `timed_out` on the outcome.
- [x] 3.3 `src/gitops/mod.rs`: `forge test` converges onto the shared runner; `run_shell_command` and `SUBPROCESS_WAIT_TIMEOUT_SECS` are deleted.
- [x] 3.4 The build step's failure reason names the timeout when the outcome timed out.
- [x] 3.5 The tests pin the timeout, the tree kill and the convergence.

## 4. Verify

- [x] 4.1 A real `forge test` on a project whose test command overruns the bound refuses with a timeout reason.
- [x] 4.2 The process the overrunning command spawned is gone with it.
- [x] 4.3 A real `forge test` on a project whose test command fails still reports the ordinary failure.
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

The bound was exercised twice: through the runner's bound-explicit entry
points with a 300 ms bound, and by a real run that waited the product's full
1800 s window. `NATIVE_COMMAND_TIMEOUT` is the product's bound and every
product path (`forge test`, the build step, the generator's verification and
the readiness fixtures) binds it.

| Check | Result | Evidence |
|---|---|---|
| The change package validates | PASS | `openspec validate --all --strict --no-interactive` → `✓ change/one-native-command-runner`, `Totals: 137 passed, 0 failed (137 items)` |
| The implementation | PASS | `cargo build` → `Finished dev profile`. The shared runner (`src/native/mod.rs`: the bound, the group spawn, the group kill, `run_native_shell`, `timed_out`), the process-group lifecycle (`src/process.rs`: `spawn_in_own_group`, `kill_process_tree`), the converged `forge test` (`src/gitops/mod.rs`) and the timeout-aware callers (`src/build/mod.rs`, `src/generate/engine.rs`, `src/readiness/mod.rs`) are in place. `SUBPROCESS_WAIT_TIMEOUT_SECS` and `run_shell_command` are gone. |
| The repository test suite | PASS, 4036 tests / 0 failed | The owner's run: `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` exited 0 over 209 test binaries: 4036 passed, 0 failed. The 4019-test baseline plus the 17 new tests. The implementer's scoped runs: `cargo test -p forge --lib native::` → 9 passed, 0 failed; `cargo test -p forge --lib gitops` → 7 passed, 0 failed; `cargo test -p forge --lib build::` → 8 passed, 0 failed; `cargo test --test native_command_runner_contract` → 15 passed, 0 failed; `cargo test --test gitops_contract` → 8 passed, 0 failed; `cargo test --test release_contract` → 10 passed, 0 failed; `cargo test --test release_gate_mandatory_contract` → 5 passed, 0 failed; `cargo test --test readiness_contract` → 10 passed, 0 failed; `cargo test --test generate_contract` → 12 passed, 0 failed. |
| The repository Gate | PASS, 8/8 | `forge gate --dry-run` resolved all 8 required checks with none unbound. The full `forge gate --timeout-secs 1800` run exited 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-21T07:14:47Z"` on revision `df37dc8`, with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. `source-file-size` evaluated 456 of 456 candidates with a maximum of 1000 physical lines. Two earlier runs were blocked by the machine, not by this change: the `security` check exceeded its 60 s deadline while `cargo audit` fetched the advisory database over the network, and the `tests` and `source-file-size` checks exceeded their deadlines while another workspace on the same machine was compiling (load average above 20). The commands are the repository's own suite and its own audit, and they pass standalone. The Gate's per-check deadlines come from the generated `.ai-gate/gate.yaml` and were not edited. |
| Real runs | PASS — the product's own 1800 s bound, waited out | The owner's run: a `rust-web` project whose test command sleeps for an hour was run through `forge test`. After 1800 s the operation refused with `error[test-failed]: test command failed: test command `cargo test` for profile `rust-web` exceeded the 1800s timeout; the command's process group was killed`, exit 1. `ps` found no surviving `cargo test`, `rustc` or fixture process afterwards: the group kill took the tree. The ordinary failure still reads as before: `error[test-failed]: test command failed: test command `cargo test` for project `native-fail` exited with status 101; see evidence: stdout: ...`, and a project whose tests pass exits 0. The implementer's scoped runs (300 ms bound) additionally pin the descendant marker never being written, the shell semantics, the redaction and the start failure. |
| OpenSpec, names, whitespace | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 137 passed, 0 failed (137 items)`; `node scripts/check-openspec-change-names.mjs` → `check-openspec-change-names: PASS`; `git diff --check` → clean |
| Lint and formatting | PASS | `rustfmt --edition 2021 --check` is clean on the eight touched files (`src/process.rs`, `src/native/mod.rs`, `src/native/native_tests.rs`, `src/gitops/mod.rs`, `src/build/mod.rs`, `src/generate/engine.rs`, `src/readiness/mod.rs`, `tests/native_command_runner_contract.rs`); `cargo clippy --workspace --all-targets --all-features` reports no warning in the touched files. The pre-existing warnings in `src/registry/model.rs`, `src/cli/auth.rs`, `tests/origin_repository_retirement_contract.rs` and `tests/deploy_ssh_adapter_contract.rs` are untouched, and no pre-existing rustfmt-dirty file was formatted. |
