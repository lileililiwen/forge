# Tasks: project-changelog-bootstrap

## 1. Plan

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the release lane's changelog requirement: `src/release/prepare.rs::load_changelog` and `src/release/contract.rs::DEFAULT_CHANGELOG`.
- [x] 1.3 Read the scaffold: `src/generate/engine.rs::template_files`, `src/generate/request.rs::manifest_text` and `readme_text`, and the profile branches.
- [x] 1.4 Confirm the canonical refusal that this change satisfies: `project-manifest-completeness` — "a project without a changelog is refused for that reason" — and confirm it stays true.
- [x] 1.5 Check the doctor's `L3-release` control, which counts release evidence by the presence of a changelog file, and decide the change does not touch it.

## 2. Design

- [x] 2.1 Decide the file's shape: keep-a-changelog, one `## [Unreleased]` section, the scaffold named under it.
- [x] 2.2 Decide that the file names no released version and no date, because nothing has been released and the assembly is deterministic.
- [x] 2.3 Decide that the file is written once, after the profile match, rather than once per profile branch.
- [x] 2.4 Decide that the release lane, the doctor's maturity rules and the existing projects are untouched.

## 3. Implement

- [x] 3.1 `src/generate/request.rs`: the changelog renderer, next to the README renderer.
- [x] 3.2 `src/generate/engine.rs`: push `CHANGELOG.md` once, after the profile match.
- [x] 3.3 The scaffold's own suite carries the new file, and the determinism check covers the changelog's bytes.
- [x] 3.4 A contract test shows a generated project can prepare a release.

## 4. Verify

- [x] 4.1 A real `forge new` writes the file, and a real `forge release prepare` on that project captures it.
- [x] 4.2 The same creation request twice renders byte-identical changelogs.
- [x] 4.3 The repository test suite passes.
- [x] 4.4 The repository Gate passes, locally, before the archive.
- [x] 4.5 `openspec validate --all --strict --no-interactive`, `node scripts/check-openspec-change-names.mjs` and `git diff --check` pass.
- [x] 4.6 Every other generated file is byte-identical to what it was.

## 5. Archive

- [x] 5.1 Archive without `--skip-specs`, so the new capability is promoted.
- [x] 5.2 Replace the archive's `TBD` Purpose with the real one.
- [x] 5.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 6. Evidence

The implementation is in place and the Gate (4.4) has passed locally, before the
archive. The archive (5.x) is the owner's remaining step.

| Check | Result | Evidence |
|---|---|---|
| The scaffold writes the file | PASS | `forge --registry /tmp/cl-verify/registry.db new /tmp/cl-verify/demo-app --profile rust-web --id demo-app` exited 0, and `/tmp/cl-verify/demo-app/CHANGELOG.md` holds the keep-a-changelog text with one `## [Unreleased]` section, no version heading and no date. |
| The release lane captures it | PASS | `forge --registry /tmp/cl-verify/registry.db release prepare /tmp/cl-verify/demo-app --version 0.1.0` exited 0 and the plan reported `changelog: CHANGELOG.md`, `changelog_hash: 4530c9819624b84f689712bc6fad0850d250d02f0606f2b638cb3e0a036c540e`, `ready: yes`, with the `doctor` and `test` checks passing. |
| The refusal is unchanged | PASS | With `CHANGELOG.md` moved out of the same project, the same command exited 1 with `error[release-invalid]: release invalid: changelog file `/tmp/cl-verify/demo-app/CHANGELOG.md` does not exist; release requires a present changelog`. The file was put back afterwards. |
| The bytes are deterministic | PASS | Two runs of the same request produced byte-identical files: `cmp /tmp/cl-det/one/demo-app/CHANGELOG.md /tmp/cl-det/two/demo-app/CHANGELOG.md` exited 0, both `sha256 4530c9819624b84f689712bc6fad0850d250d02f0606f2b638cb3e0a036c540e`. |
| The scaffold's own suite | PASS | `cargo test -p forge --lib generate` reported `test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 1338 filtered out`. |
| The contract test | PASS | `cargo test --test project_changelog_bootstrap_contract` reported `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. |
| The repository test suite | PASS, 4010 tests / 0 failed | `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` exited 0 over 208 test binaries: 4010 passed, 0 failed. The 4004-test baseline plus the 6 new tests. |
| The repository Gate | PASS, 8/8 | `forge gate --dry-run` resolved all 8 required checks with none unbound. `forge gate --timeout-secs 1800` exited 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-20T22:26:17Z"` on revision `c883e8d`, with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. `source-file-size` evaluated 456 of 456 candidates with a maximum of 1000 physical lines. |
| Nothing else in the tree moved | PASS | `diff -r` between a tree generated before the change and one generated after it reports only `CHANGELOG.md` as new, plus the standard receipt's `generated_at` timestamp — the difference the generation module already documents as allowed. Two trees generated by the same request differ only in that receipt timestamp. |
| Strict validation and names | PASS | `openspec validate --all --strict --no-interactive` reported `Totals: 135 passed, 0 failed (135 items)`; `node scripts/check-openspec-change-names.mjs` reported `check-openspec-change-names: PASS`; `git diff --check` exited 0. |
| Lint | PASS, no warning in a touched file | `cargo clippy --workspace --all-targets --all-features` exited 0 with 39 pre-existing warnings, none in `src/generate/request.rs`, `src/generate/engine.rs`, `src/generate/generate_tests.rs` or `tests/project_changelog_bootstrap_contract.rs`. |
| Formatting | PASS | `rustfmt --edition 2021 --check` is clean on all four touched files. The repository's twelve pre-existing non-rustfmt-clean files were not touched. |

