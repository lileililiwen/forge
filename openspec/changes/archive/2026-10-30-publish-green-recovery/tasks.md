## 1. BFS — Baseline and impact coverage

- [x] 1.1 Read the three CI failures and their exact causes.
- [x] 1.2 Reproduce the extractor on the current template with Python 3.12 semantics.
- [x] 1.3 Confirm readiness renders text only while `forge new` writes the feed.
- [x] 1.4 Confirm the archive layout `package.sh` produces.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Replace the invalid regex with a line-based block extractor.
- [x] 2.2 Add `render_feed_files` and append it in readiness.
- [x] 2.3 Add the readiness feed regression test.
- [x] 2.4 Allow the documented archive shape in the artifact job.
- [x] 2.5 Install the pinned validator in `platform-dotnet-quality`.
- [x] 2.6 Keep `src/readiness/mod.rs` under the source-size baseline by extracting its tests.
- [x] 2.7 Resolve the source checkout from the compile-time manifest when `target/` is relocated.
- [x] 2.8 Resolve concern checker scripts from the checkout when the executable is relocated.
- [x] 2.9 Resolve the runtime-template bundle from the checkout when the executable is relocated.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 The extractor yields the same step scripts and they pass `sh -n`.
- [x] 3.2 The readiness fixture carries a non-empty `.nupkg`.
- [x] 3.3 A synthetic archive listing accepts the share tree and rejects a bad path.
- [x] 3.4 `forge new` and the kit feed are unchanged.

## 4. Verification

- [x] 4.1 Run the repository test suite.
- [x] 4.2 Run formatting, clippy, OpenSpec validation and whitespace checks.
- [x] 4.3 Run `forge gate --dry-run`, then the full Gate.
- [x] 4.4 Record every command's exact result in the evidence table.
- [x] 4.5 Archive without `--skip-specs`, commit, then push for the next CI run.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Packaging extractor | PASS | The Python 3.12 failure was `re.error: cannot refer to an open group at position 34`. The line-based replacement produced 5 `run:` scripts from the current template; all passed `sh -n`. |
| Readiness feed | PASS | `cargo test -p forge --lib readiness::` reported 10 passed, including `readiness_fixture_includes_the_committed_feed`; `forge readiness check --profile aspnet-web` passed. The fixture now writes `packages/platform-feed/*.nupkg` before the native build. |
| Artifact shape | PASS | A synthetic listing of root files plus `share/forge/**` produced no unexpected paths; `forge-.../evil/../x` was still reported. |
| Dotnet quality workflow | PASS | `platform-dotnet-quality.yml` now sets up Node 24 and installs `@fission-ai/openspec@1.6.0` before `quality-gate.sh`; the YAML parses. |
| Relocated executable | PASS | `source_root` falls back to `CARGO_MANIFEST_DIR`; `cargo test -p forge --lib module_catalog` passed and `forge module list` returns `dotnet-platform-libs` and `identity-providers`. The concern and runtime-template resolvers prefer the source resource; `governance_concern_cli` 7 passed and `runtime_template_cli` 8 passed. |
| Source-size baseline | PASS | `src/readiness/mod.rs` was 1,007 lines; its tests moved to `src/readiness/readiness_tests.rs`, leaving 854. `src/module_catalog.rs` is 177. |
| Repository test suite | PASS, 4361 tests / 0 failed | `cargo test --workspace --no-fail-fast -- --skip ... --test-threads=1` exited 0 across 250 suites (the Gate's `tests` check then ran the all-targets script and passed too). |
| Formatting and lints | PASS | `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` exited 0. |
| Repository Gate | PASS, 8/8 | `forge gate . --timeout-secs 1800` exited 0 with aggregate `passed` at `2026-10-30T22:25:30Z`. |
| OpenSpec and whitespace | PASS | Name preflight `PASS`; strict validation `173 passed, 0 failed`; `check-spec-governance` `PASS`; `git diff --check` clean. |

