# Tasks: profile-build-command-runnable

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the profile catalogue: `src/profile/catalog.rs` and every supported profile's declared build command and toolchain.
- [x] 1.3 Read the build lane: `src/build/mod.rs` and the readiness matrix's native row in `src/readiness/mod.rs`.
- [x] 1.4 Read the generated python scaffold: `src/generate/engine.rs` and the README it renders.
- [x] 1.5 Read the repository's existing native checks: `tests/generate_contract.rs` (the per-profile build and test checks and their toolchain skips).
- [x] 1.6 Read the pinned build-command table: `tests/profile_build_command_truth_contract.rs`.
- [x] 1.7 Read the canonical capability this change modifies: `deterministic-project-generation`, and `requirement.md` §9 and §15.

## 2. DFS requirement implementation

- [x] 2.1 Decide the python-service build command: the standard library compile of the source roots the profile's layout declares.
- [x] 2.2 Decide that the build is not a packaging build and that `pyproject.toml` is untouched.
- [x] 2.3 Decide the guard: the build command's program is the declared toolchain, and the declared command runs in a generated project through Forge's own build lane.
- [x] 2.4 Decide that a profile whose toolchain the host lacks is skipped in the house's way and never claimed as passing.

## 3. Implement

- [x] 3.1 `src/profile/catalog.rs`: the `python-service` build command becomes `python3 -m compileall -q app tests`.
- [x] 3.2 `docs/release-readiness.md`: the `python-service` prerequisite line names the two commands and says the row needs only the `python3` toolchain.
- [x] 3.3 `tests/profile_build_command_truth_contract.rs`: the pinned `python-service` row carries the new build command.
- [x] 3.4 `tests/profile_build_command_runnable_contract.rs`: the program rule and the runnable rule, for every supported profile whose toolchain the host has.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 `forge build` on a generated `python-service` project runs `python3 -m compileall -q app tests` and passes.
- [x] 4.4 The generated python project's README names that command.
- [x] 4.5 `forge build` on the generated projects of the other five profiles still passes.
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
profile-build-command-runnable --yes` (no `--skip-specs`), so
`openspec/specs/profile-build-command-runnable/spec.md` was created and
`openspec/specs/deterministic-project-generation/spec.md` was updated with
every canonical scenario kept. The archive's `TBD` Purpose was replaced by
hand with the paragraph the capability's own requirements describe, and the
modified capability's Purpose now states that a generated project builds
with the stack toolchain its profile declares. The implementation and its
specs are committed together, and the handoff update is a separate
commit.

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed.

| Check | Result | Evidence |
|---|---|---|
| 4.1 `cargo build --bin forge` | pass | ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.23s`` (exit 0). The rebuilt binary is `target/debug/forge`. |
| 4.2 the repository test suite | pass | `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` → exit 0 in `real 4m19.900s`; 213 test binaries ran (213 `test result:` lines), **4060 passed, 0 failed, 1 ignored**; `grep -c FAILED` = 0. The one ignored test is pre-existing and unrelated (`forge` lib: 59 passed / 1 ignored). |
| 3.4 `cargo test --test profile_build_command_runnable_contract` | pass | `running 3 tests` → `the_profile_table_covers_every_supported_profile ... ok`, `the_build_command_runs_the_profiles_own_toolchain ... ok`, `every_declared_build_command_runs_in_a_generated_project ... ok`; `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.37s`. All six profiles were exercised on this host (the check asserts `exercised == available`, i.e. every profile whose toolchain is present). Negative control: restoring `python3 -m build` in the catalogue made the runnable rule fail naming the profile — ``profile `python-service` must build with `python3 -m build`: error[build-failed]: build command failed: build command `python3 -m build` for project `python-app` exited with status 1; see evidence: stderr: /usr/bin/python3: No module named build.__main__; 'build' is a package and cannot be directly executed``; restoring `cargo build` as `react-web`'s command made the program rule fail — ``profile `react-web` must run its own toolchain `npm` as the program of its build command `cargo build`, not `cargo` ``. The catalogue was restored after both controls. |
| 3.3 `cargo test --test profile_build_command_truth_contract` | pass | `running 6 tests` → all six `ok`; `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`. |
| 4.3 `forge build` on a generated python-service project | pass | Before the change (`forge new /tmp/dbdt19/app --profile python-service --id dbdt-python`, then `forge build /tmp/dbdt19/app`): ``error[build-failed]: build command failed: build command `python3 -m build` for project `dbdt-python` exited with status 1; see evidence: stderr: /usr/bin/python3: No module named build.__main__; 'build' is a package and cannot be directly executed`` (exit 1). After the change, the same project: `project: dbdt-python` / `profile: python-service` / `command: python3 -m compileall -q app tests` / `target: host` / `status: passed` / `exit_code: 0` / ``note: build command `python3 -m compileall -q app tests` passed for project `dbdt-python``` (exit 0). A freshly generated project (`--registry /tmp/dbdt19/profiles/registry.db new /tmp/dbdt19/profiles/python-service --profile python-service`) reports the same: `command: python3 -m compileall -q app tests`, `status: passed`, `exit_code: 0`. |
| 4.4 the generated python README | pass | `/tmp/dbdt19/profiles/python-service/README.md` → `## Native commands` / ```` ```sh ```` / `python3 -m compileall -q app tests` / `python3 -m unittest discover -s tests -v` / ```` ``` ````. The generated block is rendered from the descriptor, and the "dependency-free so native commands run without network" claim now holds for the build as well as the test. |
| 4.5 `forge build` on the other five profiles | pass | `forge --registry /tmp/dbdt19/profiles/registry.db build <dir>`: `rust-web` → `command: cargo build`, `status: passed`, `exit_code: 0`; `nextjs-web` → `command: npm run build`, `status: passed`, `exit_code: 0`; `react-web` → `command: npm run build`, `status: passed`, `exit_code: 0`; `flutter-app` → `command: flutter build bundle`, `status: passed`, `exit_code: 0`; `aspnet-web` → `command: dotnet build`, `status: passed`, `exit_code: 0`. |
| 4.6 `rustfmt --edition 2021 --check` on every touched Rust file | pass | `rustfmt --edition 2021 --check src/profile/catalog.rs tests/profile_build_command_truth_contract.rs tests/profile_build_command_runnable_contract.rs` → exit 0, no output. No pre-existing rustfmt-dirty file was touched: `git status --short` lists only `HANDOFF.md` (modified before this work started), `docs/release-readiness.md`, `src/profile/catalog.rs`, `tests/profile_build_command_truth_contract.rs`, the untracked change package and the new `tests/profile_build_command_runnable_contract.rs`; `crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `src/cli/use_cmd.rs`, `tests/delivery_cross_surface.rs` and `tests/forge_web_project_delivery_contract.rs` are untouched. |
| 4.7 `cargo clippy --workspace --all-targets --all-features` | pass | exit 0; 39 `^warning:` lines (29 individual lints plus 10 per-crate summary lines) — the pre-existing baseline; 0 errors. The warning files are `crates/cargo-audit-boost/src/parser.rs`, `crates/cargo-audit-boost/src/ui.rs`, `crates/cargo-sonar/src/analyzer.rs`, `src/hosted_ci/store.rs`, `tests/deploy_ssh_adapter_contract.rs`, `tests/origin_repository_retirement_contract.rs` — none of them a file this change touches. |
| 4.8 the source file line counts of every touched file | pass | `src/profile/catalog.rs` 654; `docs/release-readiness.md` 101; `tests/profile_build_command_truth_contract.rs` 115; `tests/profile_build_command_runnable_contract.rs` 164; `openspec/changes/profile-build-command-runnable/tasks.md` 73 — all under the 1000 physical line limit. The Gate's own `source-file-size` check agrees: `evaluated 456 of 456 candidate file(s); maximum 1000 physical line(s)`. |
| 5.1 `forge gate --dry-run` | pass | exit 0; `dry-run: gate plan rehearsal for forge (nothing was executed, persisted or journaled)`, `checks (8 planned)`: `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests` — all `[required]`, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. |
| 5.2 `forge gate --timeout-secs 1800` | pass | exit 0; `aggregate: passed`, `freshness: fresh`, `profile: full`, `runtime: forge (forge 0.1.1)`, `revision: 644b02040db5`, `observed_at: 2026-10-22T00:04:31Z`, `manifest_digest: sha256:2c456f0e82a570fb`, `rule_pack_version: forge-baseline/1.0.0`. All eight checks report `pass`: `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` (`evaluated 456 of 456 candidate file(s); maximum 1000 physical line(s)`), `tests`. Persisted to `.forge/gate/forge/evidence.json` (`"aggregate": "passed"`, `"revision": "644b02040db529cc125fe956d2de69586e32382d"`, `"observed_at": "2026-10-22T00:04:31Z"`, `"dry_run": false`). The tree was warmed first with `cargo build --workspace && cargo test --workspace --no-run` (exit 0). |
| 5.3 `openspec validate --all --strict --no-interactive` | pass | exit 0; `Totals: 141 passed, 0 failed (141 items)`. Run as `node /home/paul/.nvm/versions/node/v24.18.0/lib/node_modules/@fission-ai/openspec/bin/openspec.js validate --all --strict --no-interactive` because the `openspec` launcher on `PATH` is a broken 0-byte file. |
| 5.4 `node scripts/check-openspec-change-names.mjs` | pass | `check-openspec-change-names: PASS` (exit 0). |
| 5.5 `git diff --check` | pass | exit 0, no output. The new untracked file `tests/profile_build_command_runnable_contract.rs` has no trailing whitespace (`grep -nP '[ \t]+$'` → no match). |
