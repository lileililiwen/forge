# Tasks: Browser project creation, import and registration

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and the three delta specs agree, and a
      separate implementer could proceed without inventing architecture or an
      owner.
- [x] Read `src/api/admin.rs` (`guarded`, `is_json`, `authoring_descriptor`,
      `authoring_digest`, `authoring_write`, `deploy_id_gate`, `scrub_json`,
      `redact_local_paths`), `src/main.rs` `cmd_register`/`cmd_import`/
      `cmd_new`, `src/generate/mod.rs` (`normalize_explicit`, `generate`),
      `src/import/mod.rs` (`inspect_import`, `adopt_import`),
      `src/registry/mod.rs` (`register`), and
      `src/api/command_catalog.rs` (`web_exec`, `IMPLEMENTED_WEB_ROUTES`, the
      pinned web-list / `executable_ids` tests) as the reuse contract.
- [x] Map each requirement and scenario to its route, gate, catalog row,
      frontend render path and contract-test assertion.
- [x] Add `tests/forge_web_project_management_contract.rs` with the ten
      scenarios from design §7, serialized on one `SERIAL` mutex because
      `FORGE_ADMIN_PROJECTS_ROOT` is process-global.
- [x] Record the exact toolchain (Rust 2021, `rustc 1.87` floor) and the
      project-local ownership (extend `src/api/admin.rs`, `src/api/mod.rs`,
      `src/api/command_catalog.rs`, `frontend/`; reuse
      `src/generate`/`src/import`/`src/registry` unchanged).

## 2. DFS — Requirement-by-requirement implementation

- [x] `src/api/admin.rs`: add `ROUTE_ADMIN_PROJECT_NEW`,
      `ROUTE_ADMIN_PROJECT_IMPORT`, `ROUTE_ADMIN_PROJECT_REGISTER`,
      `ADMIN_PROJECTS_ROOT_ENV` (`FORGE_ADMIN_PROJECTS_ROOT`), the
      `ProjectManagement` enum, `management_descriptor`, `projects_root`,
      the three read-only path-free preview builders, and `management_write`
      (415 gate, `guarded`, typed-field validation, `authoring_digest`,
      no-confirm preview, mismatch `409`, confirmed
      `run_with_operation` delegation to the Core functions, scrubbed
      responses).
- [x] `src/api/mod.rs`: add `Route::AdminProjectNew`/`AdminProjectImport`/
      `AdminProjectRegister`, the three `POST ["v1","admin","projects", …]`
      router arms (the existing four-segment OPTIONS arm already covers their
      preflight), the admin short-circuit group entries and the unreachable
      dispatch arms.
- [x] `src/api/command_catalog.rs`: recatalogue `register`, `import` and
      `new` from `cli_only` to `web_exec` with the routes and typed
      parameters; add the three routes to `IMPLEMENTED_WEB_ROUTES`; update the
      pinned web list, `executable_ids` and typed-parameter map in the
      in-source tests (count stays 227).
- [x] `frontend/app.js` + `frontend/index.html`: generalize
      `buildActionControl` to a route without `{id}` and render a
      dashboard-level "Create or adopt a project" section from the catalog's
      global `execution` rows; `textContent` only, no new sink.
- [x] Implement the delta specs' scenarios alongside the code above (each
      scenario has a matching assertion in the contract test).

## 3. BFS — Cross-surface regression and completeness

- [x] Verify the workbench `upgrade` routes stay served and unchanged (the
      `upgrade` row still points at `GET /v1/admin/projects/{id}/plan`).
- [x] Verify the bearer `POST /v1/projects` creation route, the MCP
      path-based create/import tools, and the CLI dispatch are unchanged and
      still tested.
- [x] Verify every existing executable row still renders and runs; the
      command-catalog, command-execution, project-actions and workbench
      contract files pass.
- [x] Verify no response from the three new routes can carry the configured
      root, the destination path, a credential or an echoed hostile field; a
      missing root and a failing Core function are honest typed outcomes,
      never a fake success.
- [x] Remove any current-change placeholder; confirm no Core
      generate/import/registry file, CLI dispatch or `API_CONTRACT_VERSION`
      was modified beyond the intended `admin.rs` / `mod.rs` /
      `command_catalog.rs` / contract-test allowlist surfaces.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] `cargo test` green for: `forge_web_project_management_contract` (new),
      `forge_web_command_catalog_contract` (strict allowlist + `web_ids`
      updated), `forge_web_command_execution_contract`,
      `forge_web_project_actions_contract`,
      `forge_web_project_workbench_contract`, `forge_admin_api_contract`,
      `forge_web_fleet_contract`, `forge_web_publish_fleet_contract`,
      `forge_portal_frontend_contract`, `portal_ui_contract`,
      `catalog_contract`, `cargo test --lib api::` and `cargo test --bin
      forge`. Record the actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and
      `openspec validate --all --strict --no-interactive` 0 failures with
      this change active.
- [x] `git diff --check` clean; review newly added files.
- [x] Archive with `openspec archive forge-web-project-management --yes`
      (no `--skip-specs`); promote the canonical specs; update HANDOFF with
      the evidence and no `current_spec` line.
