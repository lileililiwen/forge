## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm the vacuous pass: both the prepare step and the apply step judge readiness with "every applicable check passes", and an empty applicable set makes that judgement true.
- [x] 1.2 Confirm that `release.checks` defaults to empty, so a manifest that omits the list has no applicable check.
- [x] 1.3 Confirm the two call sites and every other reader of `PlanReport.ready` and of the release report.
- [x] 1.4 Confirm the error codes, the check kinds and their statuses, the stage list and the release identity rules, none of which this change touches.
- [x] 1.5 Confirm that `forge publish` has no release-check coupling today, and that the publish lane is the `single-delivery-lane` package rather than this one.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Hoist the four known check kinds into one module constant in the release engine and use it where the kinds are captured.
- [x] 2.2 Add the readiness judgement: a release is ready only when at least one check is applicable and every applicable check passes.
- [x] 2.3 Report the absence of an applicable check as its own reason, and name the four check kinds in it.
- [x] 2.4 Use the judgement in the prepare step, so a plan with no applicable check is not ready and reports no passing check count.
- [x] 2.5 Use the same judgement in the apply step, so a release with no applicable check is refused before any stage runs.
- [x] 2.6 Keep the existing plan note and refusal shapes for a project whose check fails.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Check that a project with a passing check still produces a ready plan and still walks the stages on apply.
- [x] 3.2 Check that a project whose check fails, is unavailable or is stale is still refused, with the failing check named.
- [x] 3.3 Check that a project with no `release` block, and a project with no changelog, are still refused by the earlier refusals.
- [x] 3.4 Check that the apply step without `--confirm` still refuses with the confirm refusal.
- [x] 3.5 Check that the release error codes, the check kinds, the statuses, the stages, the identity rules and the retry semantics are unchanged.
- [x] 3.6 Check that the `prepare-release` procedure description matches the new behaviour.
- [x] 3.7 Check that no sibling module is edited.

## 4. Verification

- [x] 4.1 Run the repository test suite: `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1`.
- [x] 4.2 Run the repository Gate: the `forge gate --dry-run` rehearsal and the full `forge gate`.
- [x] 4.3 Run `openspec validate --all --strict --no-interactive` and `git diff --check`.
- [x] 4.4 Prepare a release for a real project that declares no check and show that the plan is not ready, that the reason names the missing check, and that the apply step refuses without creating a tag.

## 5. Implementation — authorized surfaces (2026-10-20)

- [x] 5.1 `src/release/engine/prepare.rs`.
- [x] 5.2 `src/release/engine/apply.rs`.
- [x] 5.3 `src/release/engine/engine_tests.rs`.
- [x] 5.4 `src/procedure/catalog.rs`.
- [x] 5.5 `tests/forge_web_project_release_contract.rs`, whose fixture relied on the vacuous pass. Found while implementing; added to the authorized surfaces.
- [x] 5.6 Record the exact output of every verification command in the evidence table below.
- [x] 5.7 Archive without `--skip-specs` so the capability is promoted into `openspec/specs/release-evidence-non-vacuous/spec.md`, commit the implementation with its specs, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Repository test suite | PASS | `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` exited 0 and reported 3888 passed, 0 failed. `cargo build` finished with no finding. The focused run `cargo test -p forge --lib release::` reported 30 passed, 0 failed; `cargo test -p forge --test forge_web_project_release_contract` reported 8 passed, 0 failed. |
| Repository Gate | PASS | `forge gate --dry-run` resolved all 8 required checks with none unbound. The full `forge gate --timeout-secs 1800` run started at 12:49 and finished at 12:57 with exit code 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-20T04:57:46Z"` on revision `9e60736`, with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. The `source-file-size` check reported "evaluated 439 of 439 candidate file(s); maximum 1000 physical line(s)". The `security` check reported the pre-existing unmaintained-crate advisories for `paste` and `unsafe-libyaml` and still passed. |
| Strict OpenSpec validation | PASS | `node scripts/check-openspec-change-names.mjs` reports `PASS`; `openspec validate --all --strict --no-interactive` reported `Totals: 127 passed, 0 failed (127 items)`. |
| Whitespace | PASS | `git diff --check` reported no findings. All five changed Rust files are rustfmt-clean under `rustfmt --edition 2021 --check`; the twelve files that already differed under rustfmt before this change were left alone and `cargo fmt --all` was not run. |
| Real release | PASS | A project whose manifest declares a `release:` block with no `checks:` list was prepared for real: `forge release prepare --version 1.0.0 --dry-run` reported `ready: no` and the summary "release plan not ready: no applicable release check; declare at least one of doctor, test, driftwatch, gate in release.checks; release execution blocks before tag or publication", with all four checks reported as `disabled applicable=false`. `forge release apply --version 1.0.0 --confirm` refused with `error[release-check-failed]`, exited 1, and left no tag, no release state and no new commit. A second project that declares the `doctor` and `test` checks was prepared for real and reported `ready: yes` with "2 check(s) passed". |
