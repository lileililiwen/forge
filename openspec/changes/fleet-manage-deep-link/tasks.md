# Tasks: Fleet per-project Manage entries land on the managing view

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Map each requirement/scenario to its `frontend/app.js` symbol (`applyWorkbenchProjectParam`, `openInWorkbench`, `applyManagementProjectParam` redirect arm, `loginNextTarget`), fleet/manage/workbench/login call sites, and test assertion.
- [x] Read `frontend/app.js` router + `loginPage` + `dashboardPage` + `initWorkbench` + `loadWorkbenchDetail`, `frontend/login.html`, `tests/forge_web_manage_deep_link_browser.rs` + `tests/browser/manage-deep-link-check.mjs` (house harness to mirror), and `tests/forge_web_navigation_contract.rs` (token-test style).
- [x] Record alethefy's live state and the browser repro (observed/unmanaged via `published`, onboardable, fleet Manage → `/management?project=alethefy`; signed-out exact URL loses the param across login; workbench `?project=` unsupported) without changing live data.
- [x] Verify the implementation-handoff gate: language (vanilla JS, no build), project (vendored `frontend/`), shared-library (none — project-local), UI/UX boundary (fleet rows, workbench, management, login).

## 2. DFS — Requirement-by-requirement implementation

- [x] `frontend/app.js`: `workbenchProjectParam` + `applyWorkbenchProjectParam` with `wbAutoParam` bookkeeping; call from `renderRoute` (workbench view) and at the end of `initWorkbench`; unknown id → honest notice, nothing clobbered; absent id → no-op.
- [x] `frontend/app.js`: `openInWorkbench` navigates to `/workbench?project=<encoded identity>`; loading is applied by the route render.
- [x] `frontend/app.js`: `applyManagementProjectParam` redirects a `registered`-state match to `/workbench?project=<id>`; all other non-selectable/unknown states keep the select-nothing notice.
- [x] `frontend/app.js`: `loginNextTarget` (same-origin + `isRoutePath` validated); `dashboardPage` carries `?next=` to `login.html`; `loginPage` returns to `next` (already-authenticated and post-submit), else `index.html`.
- [x] `frontend/index.html` + `frontend/app.js`: project-scoped management card (`#mgmt-project` leads, `#ws-bulk` hides) with `mgmtHasProjectParam` / `mgmtScopedRender` / `mgmtScopedPreview` / `mgmtScopedRun`; onboardable → scoped summary + single-project preview/confirm/run; registered → workbench redirect; unknown/empty/non-selectable → scoped safe notice, no bulk tick, no preview; plain `/management` bulk-exact.
- [x] Delta spec scenarios implemented alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify plain `/workbench`, `/management`, `login.html` behavior unchanged; hand selections never clobbered; no `#` navigation introduced.
- [x] Verify no JSON API, registry/journal, catalog row/count, CLI, static-server or style change.
- [x] Verify `next` cannot open-redirect (absolute URL, foreign origin, unknown path, `login.html` all fall back to `index.html`).
- [x] New `tests/browser/workbench-deep-link-check.mjs` + `tests/forge_web_workbench_deep_link_browser.rs` cover both halves (managed boot, reload, unknown id, back/forward, registered redirect, login round-trip).
- [x] Rework `tests/browser/manage-deep-link-check.mjs` for the scoped view (scoped leads + bulk hidden, scoped preview covers only the id, plain `/management` stays bulk, unknown safe, reload + back/forward reconcile); update the workbench harness login assertion to the scoped view.
- [x] Extend the navigation contract token test for the new symbols; existing manage deep-link browser oracle stays green.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] `cargo test --test forge_web_workbench_deep_link_browser` green (new); `cargo test --test forge_web_navigation_contract` green (extended).
- [x] `cargo test --test forge_web_manage_deep_link_browser`, `cargo test --test forge_web_command_catalog_contract`, `cargo test --test forge_web_maintainer_surface_contract`, `cargo test --lib command_catalog`, `cargo test --bin forge` green. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [ ] `forge gate --dry-run`, then `forge gate --timeout-secs 600` with the verdict recorded in HANDOFF; no new failure attributable to this change. (Runs at completion; pre-existing `source-file-size` fail and environmental adapter timeouts are recorded, not caused.)
- [ ] Leave the change active and uncommitted per operator direction (no archive, no commit in this turn).
