# Tasks: Dynamic workspace onboarding

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/api/admin.rs` (`guarded`, `is_json`, `authoring_digest`, `projects_root`, `management_destination`, `management_preview`, `scrub_json`, `run_management_import`, `run_management_register`), `src/api/mod.rs` (route variants, admin lists), `src/api/command_catalog.rs` (`IMPLEMENTED_WEB_ROUTES`, allowlist tests), `src/import/mod.rs` (`inspect_import`, `adopt_import`, `derive_project_id`), `src/registry/mod.rs` (`register`, `check_identity_available`, `record_operation`), `frontend/app.js` + `frontend/index.html` (management area, generic controls, table region pattern) as the reuse contract.
- [x] Map each requirement and scenario to its route, gate, preview/apply shape, frontend render path and contract-test assertion.
- [x] Add `tests/forge_web_workspace_onboarding_contract.rs` with the design §8 scenarios and a throwaway root (manifest/importable/ambiguous/garbage dirs), serializing on one `SERIAL` mutex because `FORGE_ADMIN_PROJECTS_ROOT` is process-global.
- [x] Add `tests/browser/workspace-onboarding-check.mjs` and `tests/forge_web_workspace_onboarding_browser.rs`, reporting unavailable browser prerequisites as `UNVERIFIED`, never a pass.
- [x] Record the exact toolchain (Rust 2021, `rustc 1.87` floor) and project-local ownership (extend `src/api/admin.rs`, `src/api/mod.rs`, `src/api/command_catalog.rs` allowlist, `frontend/`; reuse `src/import`/`src/registry` unchanged).

## 2. DFS — Requirement-by-requirement implementation

- [x] `src/api/admin.rs`: add `ROUTE_ADMIN_WORKSPACE_CANDIDATES` / `ROUTE_ADMIN_WORKSPACE_ONBOARD`, leaf validation, bounded live directory reader, candidate-state computation, per-item preview builders, bulk descriptor/digest, `workspace_candidates` read route and `workspace_onboard_write` (415 gate, `guarded`, typed-field validation, no-confirm preview, mismatch `409`, confirmed ordered apply with per-item results, `207` on partial, parent `admin.workspace.onboard` counts-only row, scrubbed responses).
- [x] `src/api/mod.rs`: add `Route::AdminWorkspaceCandidates` / `Route::AdminWorkspaceOnboard`, the two router arms, and the admin short-circuit, permission, dispatch and unreachable-list entries.
- [x] `src/api/command_catalog.rs`: add both routes to `IMPLEMENTED_WEB_ROUTES` only; keep row count at 227.
- [x] `tests/forge_web_command_catalog_contract.rs`: add both routes to the web-route allowlist (no `web_ids` change).
- [x] `frontend/app.js` + `frontend/index.html`: add the “Workspace onboarding” panel (discover/refresh, candidate table with selection, per-item profile/id overrides, preview-selected, confirm, per-item results, fleet refresh); `textContent` only, labelled controls, live-region status, no new sink.
- [x] `README.md`: document the `FORGE_ADMIN_PROJECTS_ROOT` prerequisite and the onboarding flow in the portal section.
- [x] Implement the delta spec's scenarios alongside the code above (each scenario has a matching assertion in the contract or browser test).

## 3. BFS — Cross-surface regression and completeness

- [x] Verify the CLI import/register dispatch, single-item management routes, bearer routes, catalog row count, fleet aggregation and journal behavior are unchanged and still tested.
- [x] Verify every existing executable row still renders and runs; the command-catalog, command-execution, management, workbench and fleet contract files pass.
- [x] Verify no response from the two new routes can carry the configured root, a resolved destination, a manifest body, a credential or an echoed hostile field; ambiguous candidates stay unselected with reasons; partial batches report honestly.
- [x] Verify a directory added after first discovery appears on refresh with no code or config change (dynamic proof in tests).
- [x] Remove any current-change placeholder; confirm no Core import/registry/fleet file, CLI dispatch, `API_CONTRACT_VERSION` or catalog row was modified beyond the intended surfaces.
- [x] Verify the Chromium flow against a throwaway root, including keyboard operation and rendered-path checks.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] `cargo test` green for: `forge_web_workspace_onboarding_contract` (new), `forge_web_workspace_onboarding_browser` (new), `forge_web_command_catalog_contract` (allowlist updated), `forge_web_command_execution_contract`, `forge_web_project_management_contract`, `forge_web_project_workbench_contract`, `forge_admin_api_contract`, `forge_web_project_fleet_contract`, `forge_portal_frontend_contract`, `portal_ui_contract`; `cargo test --bin forge` and `cargo test --lib api::` all pass. Record the actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [x] Archive with `openspec archive forge-web-workspace-onboarding --yes` (no `--skip-specs`); promote the canonical spec; update HANDOFF with the evidence and pointer handling.
