## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm the two rules: the delivery lane refuses without fresh, passing, full-profile Gate evidence, and the deploy lane applies an artifact with no evidence at all.
- [x] 1.2 Confirm that the publish lane has no reference to the gate anywhere, and record why it is out of scope.
- [x] 1.3 Confirm the deploy lane's first side effect and the position of the rule relative to it.
- [x] 1.4 Confirm the delivery lane's private rule, its exact conditions, its error variant and its position in the preflight.
- [x] 1.5 Confirm the release lane's own check model and record why it is out of scope.
- [x] 1.6 Confirm the Gate's evidence contract, its freshness rule and its supported contract versions, none of which this change touches.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add the shared rule to a new `src/gate/ship_gate.rs`: one function that decides whether a project may deliver a given revision, and one status value that carries the verdict and the reason.
- [x] 2.2 Register the module and re-export it from `src/gate/mod.rs`.
- [x] 2.3 Delete the delivery lane's private rule and call the shared rule instead, keeping its error variant and its position in the preflight.
- [x] 2.4 Adopt the shared rule in the deploy lane: evaluate it for the captured source revision before the first side effect and refuse when it is not satisfied.
- [x] 2.5 Keep a dry-run deployment ungated, because it contacts no target.
- [x] 2.6 Report the rule in the deployment plan: the plan carries the status, and its readiness and note follow from it.
- [x] 2.7 Keep the refusal wording in one place, so both lanes name the same reason.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Check every refusal case: no evidence, a rehearsal record, stale evidence, failed evidence, another project's evidence, a profile other than full, and an unsupported contract.
- [x] 3.2 Check that a refused deploy leaves no delivery state, no journal row and no target change.
- [x] 3.3 Check that a project with satisfied evidence deploys exactly as before, and that its plan is ready.
- [x] 3.4 Check that a dry-run deploy is not refused.
- [x] 3.5 Check that the delivery lane's stage and promote verbs still inherit the preflight's verdict, and that its own refusals are unchanged.
- [x] 3.6 Check that the Gate's evidence format, its contract version, its freshness rule and its command surface are unchanged.
- [x] 3.7 Check that the release lane, the publish lane, the deploy adapter contract and the delivery provider contract are unchanged.
- [x] 3.8 Check that no sibling module is edited.

## 4. Verification

- [x] 4.1 Run the repository test suite: `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1`.
- [x] 4.2 Run the repository Gate: the `forge gate --dry-run` rehearsal and the full `forge gate`.
- [x] 4.3 Run `openspec validate --all --strict --no-interactive` and `git diff --check`.
- [x] 4.4 Run a real deployment against a project without Gate evidence and show that the plan is not ready and that the apply refuses without touching the target.

## 5. Implementation — authorized surfaces (2026-10-20)

- [x] 5.1 `src/gate/ship_gate.rs` (new).
- [x] 5.2 `src/gate/mod.rs`.
- [x] 5.3 `src/delivery/handlers.rs`.
- [x] 5.4 `src/deploy/engine/stages.rs`.
- [x] 5.5 `src/deploy/model.rs` and `src/api/admin/deploy.rs`, whose flat browser plan view carries the rule's verdict and its reason.
- [x] 5.6 `src/api/admin/deploy_views.rs` (new) and `src/api/admin/mod.rs`. The route file was at 998 physical lines before this change and the repository caps a source file at 1000, so the path-free views moved out of it. Found by the repository Gate's `source-file-size` check.
- [x] 5.6 The deploy contract fixtures that exercise a real apply, and the new tests.
- [x] 5.7 Record the exact output of every verification command in the evidence table below.
- [x] 5.8 Archive without `--skip-specs` so the capability is promoted into `openspec/specs/single-delivery-lane/spec.md`, commit the implementation with its specs, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Repository test suite | PASS | `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` exited 0 and reported 3904 passed, 0 failed. `cargo build` finished with no finding. The focused runs: `cargo test -p forge --lib gate::` reported 56 passed, 0 failed; `cargo test -p forge --lib deploy::` reported 37 passed, 0 failed; `cargo test -p forge --lib delivery::` reported 21 passed, 0 failed; `cargo test -p forge --test deploy_contract` reported 20 passed, 0 failed; `cargo test -p forge --test delivery_contract` reported 21 passed, 0 failed. |
| Repository Gate | PASS | `forge gate --dry-run` resolved all 8 required checks with none unbound. The first full run was blocked by the `source-file-size` check: `src/api/admin/deploy.rs` had reached 1003 physical lines because the browser plan view grew by two fields, and the repository caps a source file at 1000. The views were moved to a new `src/api/admin/deploy_views.rs`, the file came back to 887 lines, and the suite was re-run. The second full `forge gate --timeout-secs 1800` run exited 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-20T06:57:53Z"` on revision `e0a15cf`, with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. The `source-file-size` check reported "evaluated 442 of 442 candidate file(s); maximum 1000 physical line(s)". The `security` check reported the pre-existing unmaintained-crate advisories for `paste` and `unsafe-libyaml` and still passed. |
| Strict OpenSpec validation | PASS | `node scripts/check-openspec-change-names.mjs` reports `PASS`; `openspec validate --all --strict --no-interactive` reported `Totals: 128 passed, 0 failed (128 items)`. |
| Whitespace | PASS | `git diff --check` reported no findings. All thirteen changed Rust files, including the new `src/gate/ship_gate.rs`, `src/gate/ship_gate_tests.rs` and `src/api/admin/deploy_views.rs`, are rustfmt-clean under `rustfmt --edition 2021 --check`; the twelve files that already differed under rustfmt before this change were left alone and `cargo fmt --all` was not run. `cargo build` reports no warning. |
| Real deployment | PASS | A project with a `local` target, a `Dockerfile` artifact and one commit: `forge deploy plan` reported `ready: no` with "deploy plan not ready: Forge Gate evidence is absent for `gated-app`; run `forge gate --profile full` from the project directory and retry". `forge deploy apply --target-name local --confirm` refused with `error[deploy-invalid]`, exited 1, and left no `.forge` directory at all, so no delivery state and no journal row. With a full-profile passed evidence record bound to the project's HEAD, the same plan reported `ready: yes` and the apply passed the rule and reached the adapter, which then failed for the unrelated reason that the `forge-deployer` binary is not installed in this environment. `forge deploy apply --dry-run` on the project without evidence was not refused by the rule and reached the same adapter. |
