## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm the two refusal paths: `forge release` refuses a project with no `release` section, `forge deploy` refuses a project with no `deployment` section, and neither is ever written by Forge.
- [x] 1.2 Confirm the two manifest writers and every third caller of `build_manifest_text`.
- [x] 1.3 Confirm that `forge.yaml` has no JSON schema, so the Rust parser is the only contract, and that the manifest schema version stays 1.
- [x] 1.4 Baseline the current state: a generated manifest carries no `release` and no `deployment` block; a registry observation reads the legacy `deployment.target` only.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add `src/core/manifest_blocks.rs` with the default release checks, the release block renderer, the deployment defaults derivation and the deployment block renderer.
- [x] 2.2 Register the module in `src/core/mod.rs`.
- [x] 2.3 Write the release block into the generated manifest.
- [x] 2.4 Write the release block into the adopted manifest.
- [x] 2.5 Derive the deployment defaults from the files the generated tree contains.
- [x] 2.6 Derive the deployment defaults from the repository's files for an adopted project.
- [x] 2.7 Write the deployment block into the generated manifest.
- [x] 2.8 Write the deployment block into the adopted manifest.
- [x] 2.9 Record the deployment target in the registry observation from the declared default, the first typed target, or the legacy field.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Check that a hand-written manifest is never rewritten and that the feature, kit, runtime and project blocks are unchanged.
- [x] 3.2 Check that the release contract, the deploy contract, the profile catalog, the doctor and the delivery lanes are unchanged.
- [x] 3.3 Check that no sibling module is edited.
- [x] 3.4 Check that the rendered manifest parses through the real parser and that both configurations resolve.

## 4. Verification

- [x] 4.1 Run the repository test suite: `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1`.
- [x] 4.2 Run the repository Gate: the `forge gate --dry-run` rehearsal and the full `forge gate`.
- [x] 4.3 Run `openspec validate --all --strict --no-interactive` and `git diff --check`.
- [x] 4.4 Generate a project for real and show that `forge release prepare` and `forge deploy plan` no longer refuse for a missing block.

## 5. Implementation — authorized surfaces (2026-10-19)

- [x] 5.1 `src/core/manifest_blocks.rs` (new) and `src/core/mod.rs`.
- [x] 5.2 `src/generate/request.rs` and `src/generate/engine.rs`.
- [x] 5.3 `src/import/detect.rs` and the graduation import call site.
- [x] 5.4 `src/registry/model.rs`.
- [x] 5.5 The tests that assert the generated and adopted manifest content, and the registry observation test.
- [x] 5.6 Record the exact output of every verification command in the evidence table below.
- [ ] 5.7 Archive without `--skip-specs` so the capability is promoted into `openspec/specs/project-manifest-completeness/spec.md`, commit the implementation with its specs, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Repository test suite | PASS | `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` reported `3884 passed, 0 failed` and exited 0. The three skipped tests are the native-toolchain scaffold tests, skipped for the same reason as before this change. The affected modules were re-run on their own: `cargo test -p forge --lib generate::` reported 29 passed, `cargo test -p forge --lib import::` reported 17 passed, `cargo test -p forge --lib registry::` reported 60 passed. |
| Repository Gate | PASS | `forge gate --dry-run` reported 8 required checks with none unbound. The full `forge gate --timeout-secs 1800` run started at 11:41 and finished at 11:55 with exit code 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` and `"observed_at": "2026-10-20T03:55:21Z"`, with all eight checks in state `pass`: `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests`. The `source-file-size` check reported "evaluated 439 of 439 candidate file(s); maximum 1000 physical line(s)". The `security` check reported the pre-existing unmaintained-crate advisories for `paste` and `unsafe-libyaml`; the check itself passed. |
| Strict OpenSpec validation | PASS | `node scripts/check-openspec-change-names.mjs` reports `PASS`; `openspec validate --all --strict --no-interactive` reported `Totals: 126 passed, 0 failed (126 items)`. |
| Whitespace | PASS | `git diff --check` reported no findings. The new module is rustfmt-clean; twelve files that differ under rustfmt were already different before this change and were left alone. |
| Real generation | PASS | A `rust-web` project was generated with the built binary. Its manifest now carries the `release` block (`versioning: semver`, checks `doctor` and `test`) and the `deployment` block (`artifact: Dockerfile`, target `local`). `forge deploy plan` returned `ready: yes` with the artifact and its hash, where it previously refused with "has no `deployment` section". `forge release prepare --version 0.1.0 --dry-run` returned `ready: yes` with two applicable checks passed and two reported as not configured, where it previously refused with "has no `release` section". The one remaining refusal is the changelog file: the release contract has always required a present `CHANGELOG.md`, and the scaffold writes none. An adopted repository got the same blocks, and `forge inspect` now names the `local` target where it reported `unknown`. |
