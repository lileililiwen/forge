# Tasks: Browser-executable staged delivery

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta specs agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/delivery/handlers.rs`, `invoke.rs`, `projection.rs` (including the connected-Hermora decoder defect), `state.rs`, `hermora.rs`, `cli.rs`, `src/api/admin.rs`, `src/api/mod.rs`, `src/api/command_catalog.rs`, `frontend/app.js`, `frontend/index.html` and the active canonical delivery/catalog/execution specs as the reuse contract.
- [x] Map every requirement and scenario to its route, gate, catalog row, frontend render path and contract-test assertion.
- [x] Add `tests/forge_web_project_delivery_contract.rs` with hermetic provider/Hermora stubs and process-global environment isolation.
- [x] Add `tests/browser/delivery-workbench-check.mjs` and `tests/forge_web_project_delivery_browser.rs`, reporting unavailable browser prerequisites as `UNVERIFIED`, never a pass.
- [x] Record the exact toolchain and ownership: extend API/catalog/frontend surfaces; reuse delivery Core unchanged.

## 2. DFS — Requirement-by-requirement implementation

- [x] Add five admin route variants, router arms, session/permission/dispatch/unreachable entries and route consts.
- [x] Correct `populate_hermora` to decode optional detail strings as JSON values, preserving `site_id` when `"reason"` is null, and add its projection unit test.
- [x] Add delivery descriptors, registered-revision resolution, path-free previews, confirm/digest gates, Core delegation, typed failure mapping and response scrubbing.
- [x] Recatalogue all five delivery rows as `web`; add executable blocks for four mutations; extend implemented routes and pinned catalog assertions.
- [x] Add the workbench delivery card, status/next-action rendering, mutation refresh and accessible loading/error/empty states.
- [x] Implement every delta-spec scenario alongside the code above, with one matching contract or browser assertion per scenario.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify CLI delivery dispatch, bearer delivery routes, provider/Hermora contracts and journal behavior are unchanged and still tested.
- [x] Verify existing publish/deploy/release/authoring/share surfaces and executable catalog rows still pass.
- [x] Verify hostile inputs are never echoed; absolute paths, adapter paths and credential-shaped values never reach the browser.
- [x] Verify no Core logic was duplicated or altered and no unrelated file was modified beyond the intended surfaces.
- [x] Verify the Chromium flow against a throwaway registry, project and stubs, including keyboard focus and rendered-path checks.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` with 0 errors.
- [x] New contract, browser oracle, catalog/execution/deployment/release/actions/workbench/admin/fleet/publish/frontend/portal tests, delivery Core suites, `--bin forge` and `--lib api::` all pass. Record actual counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` has 0 failures.
- [x] `git diff --check` clean; review newly added files.
- [x] Archive with `openspec archive forge-web-project-delivery --yes` without `--skip-specs`; promote canonical specs; update HANDOFF with evidence and pointer handling.
