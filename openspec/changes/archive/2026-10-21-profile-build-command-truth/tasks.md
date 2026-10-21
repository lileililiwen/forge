# Tasks: profile-build-command-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the profile catalogue: `src/profile/catalog.rs` and every declared build command.
- [x] 1.3 Read the descriptor doors: `src/profile/resolve.rs` (`validate_descriptor`, `inspect_profile`, `descriptor_from_yaml`).
- [x] 1.4 Read the build lane: `src/build/mod.rs` and the readiness matrix's native row.
- [x] 1.5 Read the Flutter scaffold: `src/generate/engine.rs` and `docs/release-readiness.md`.
- [x] 1.6 Read the canonical capabilities this change modifies: `profile-registry`.
- [x] 1.7 Read the house pinned-table pattern: `tests/workspace_metadata_contract.rs`.

## 2. DFS requirement implementation

- [x] 2.1 Decide that the Flutter build is `flutter build bundle` and that the app-store package stays a release-stage command.
- [x] 2.2 Decide the rule: a named list of six analysis verbs, an unknown verb accepted.
- [x] 2.3 Decide the verb: the last token that is not a flag.
- [x] 2.4 Decide the doors: `validate_descriptor` for an external document, `inspect_profile` for the built-in catalogue; the list is not a door.
- [x] 2.5 Decide the guard: the catalogue's build commands are pinned by a contract check.

## 3. Implement

- [x] 3.1 `src/profile/catalog.rs`: the `flutter-app` build command becomes `flutter build bundle`.
- [x] 3.2 `src/profile/resolve.rs`: `ANALYSIS_BUILD_VERBS`, `build_command_verb` and `check_build_command`; the rule is called at both doors.
- [x] 3.3 `src/generate/engine.rs`: the Flutter scaffold's README names the build and the release-stage package.
- [x] 3.4 `docs/release-readiness.md`: the Flutter prerequisite line names the build command the profile declares.
- [x] 3.5 `tests/profile_build_command_truth_contract.rs`: the pinned catalogue table and the refusal, through the library.
- [x] 3.6 `src/profile/resolve.rs`: unit tests for the verb reading and the refusal at both doors.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 `forge new --profile flutter-app` names `flutter build bundle` as the build command, and the generated README names it.
- [x] 4.4 `forge build` on the Flutter scaffold runs `flutter build bundle` and passes.
- [x] 4.5 A descriptor that declares `cargo clippy` as its build command is refused with `invalid-profile`, naming the profile, the command and the verb.
- [x] 4.6 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.7 `cargo clippy --workspace --all-targets --all-features` reports no new warning.

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

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed.

| Check | Result | Evidence |
|---|---|---|
| 4.1 `cargo build --bin forge` | PASS | `Finished \`dev\` profile [unoptimized + debuginfo] target(s)`; the binary is at `target/debug/forge` (the repository's own target directory) |
| 4.2 the repository test suite | PASS, 4046 tests / 0 failed | `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` exited 0 over 210 test binaries: 4046 passed, 0 failed, 1 ignored, and `grep -c "^failures:"` reports 0. The 4036-test baseline plus the 10 new tests (6 in the new contract test, 4 in the profile module). |
| 3.5 `cargo test --test profile_build_command_truth_contract` | PASS | `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| 3.6 `cargo test -p forge --lib profile::` | PASS | `test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 1361 filtered out` |
| 3.3 `cargo test --test generate_contract` | PASS | `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| 3.4 `cargo test --test workspace_metadata_contract` | PASS | `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| 4.3 `forge new /tmp/fpbct2/app --profile flutter-app --id fb-app` | PASS | exit 0; `forge profile inspect flutter-app` reports `build: flutter build bundle` and `test: flutter test`; the generated `README.md` line 9 reads `flutter build bundle`, line 10 `flutter test`, and line 13 reads ``Built with `flutter build bundle` and verified with `flutter test` (no platform host or Android SDK needed). The app store package (`flutter build appbundle`) is a release-stage command that requires `android/` and the Android SDK.`` |
| 4.4 `forge build /tmp/fpbct2/app` | PASS | exit 0; `project: fb-app`, `profile: flutter-app`, `command: flutter build bundle`, `status: passed`, `exit_code: 0`, `note: build command \`flutter build bundle\` passed for project \`fb-app\``. The scaffold really built: `build/flutter_assets/` holds `AssetManifest.bin`, `FontManifest.json`, `NOTICES.Z`, `fonts/` and `isolate_snapshot_data`. The Flutter SDK on this host is 3.47.1, and `flutter build bundle` needs neither the Android SDK nor an `android/` directory. |
| 4.5 an external descriptor with `build_command: cargo clippy` | REFUSED | `error[invalid-profile]: invalid profile descriptor: profile descriptor 'custom-web' declares the analysis command \`cargo clippy\` as its build command; \`clippy\` analyses, it does not build; declare a command that produces build output; no files were changed` — the code is `invalid-profile`, and the reason names the profile, the command and the verb. |
| 4.6 `rustfmt --edition 2021 --check` on every touched Rust file | CLEAN | `src/profile/resolve.rs`, `src/profile/catalog.rs`, `src/generate/engine.rs`, `tests/profile_build_command_truth_contract.rs` report no diff. The pre-existing dirty files (`crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `src/cli/use_cmd.rs`, `tests/delivery_cross_surface.rs`, `tests/forge_web_project_delivery_contract.rs`) are untouched: `git status --short` lists none of them. |
| 4.7 `cargo clippy --workspace --all-targets --all-features` | PASS | exit 0 with 39 warnings, the same pre-existing count as the baseline. None is in `src/profile/resolve.rs`, `src/profile/catalog.rs`, `src/generate/engine.rs` or `tests/profile_build_command_truth_contract.rs`. |
| 5.1 `forge gate --dry-run` | PASS | 8 required checks resolved, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. |
| 5.2 `forge gate --timeout-secs 1800` | PASS, 8/8 | exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-21T15:57:03Z"` on revision `b42a049c6ebceb1b0d3f0ae23d495601f25a27b2`, with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. `source-file-size` evaluated 456 of 456 candidates with a maximum of 1000 physical lines. The `security` check reports the pre-existing unmaintained-crate advisories for `paste` and `unsafe-libyaml` and still passes. |
| 5.3 `openspec validate --all --strict --no-interactive` | PASS | `Totals: 138 passed, 0 failed (138 items)` before the archive, and the same after it. |
| 5.4 `node scripts/check-openspec-change-names.mjs` | PASS | `check-openspec-change-names: PASS` |
| 5.5 `git diff --check` | CLEAN | exit 0, no output |

### One environment note, because it changed how the evidence was produced

The machine's cargo layout changed while this change was in flight: the
cargo home and the sccache cache moved to the SSD
(`$HOME/.cargo`, `$HOME/.cache/sccache`) and the global config points the
target directory at `/mnt/westdata/cargo-target`, a shared directory on the
slow spinning disk. That layout breaks four test binaries for a reason that
has nothing to do with this change: Forge resolves its module catalog and
its concern checker from the checkout the executable lives in
(`src/module_catalog.rs::source_root` walks up from `current_exe()` looking
for a `Cargo.toml` next to a `modules/` directory), so a target directory
outside the checkout makes the module catalog unavailable and the concern
checker missing. The owner's runs are therefore taken with the target
directory inside the repository, and the SSD cargo home and sccache cache,
which is the layout the previous change's suite ran in. A full rebuild was
accepted rather than working around the layout.

One consequence is recorded here rather than hidden: the first Gate run
in this window reported `build unresolved: state \`REVIEW_REQUIRED\` ...
adapter \`project-runtime\` exceeded its timeout of 60000ms`. The
`build` check runs `cargo build --workspace` and had to wait behind the two
test-bearing checks for the shared build lock, and the Gate's 60 s
per-check deadline for that check is shorter than that wait. The target
directory was warmed first (`cargo build --workspace` and `cargo test
--workspace --no-run`) and the next Gate run passed 8/8 with the build
check finishing in 1.05 s. The Gate's per-check deadlines come from the
generated `.ai-gate/gate.yaml` and were not edited.

### Notes on the implementation

- `build_command_verb` uses `split_whitespace().rfind(|t| !t.starts_with('-'))`
  instead of the `filter(..).last()` form in the design. The behaviour is the
  same — the command's last non-flag token — and `rfind` is the form clippy
  accepts: `filter(..).last()` raised `clippy::double_ended_iterator_last` and
  `filter(..).next_back()` raised `clippy::filter_next`, both inside
  `src/profile/resolve.rs`, which this change owns.
- The refusal for an external descriptor document was exercised through the
  library (`forge::profile::descriptor_from_yaml`), because no CLI or HTTP
  surface reads an external descriptor document today:
  `descriptor_from_yaml` has no caller outside `src/profile/resolve.rs` and
  the tests. The `forge profile inspect` door was exercised for the
  built-in catalogue.
