# Tasks: Read-only project status and fleet readiness

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and the two delta specs agree, and a
      separate implementer could proceed without inventing architecture or an
      owner.
- [x] Read `src/api/workbench.rs` (`resolve`, `Resolved`,
      `redact_local_paths`, `detail`/`build_health`), `src/api/admin.rs`
      (`guarded`, `handle`), `src/api/mod.rs` (route enum, `route_request`,
      `required_permission`, admin short-circuit, unreachable arms),
      `src/checker/mod.rs` (`build_document`, `truncate_alerts`,
      `readiness_alerts`, `AlertSeverity`), `src/doctor/mod.rs`
      (`run_doctor`, `DoctorReport`, `FindingStatus`), `src/governance.rs`
      (`inspect`), `src/profile/mod.rs` (`inspect_profile`,
      `ProfileSupportStatus`) and `src/readiness/mod.rs`
      (`matrix_profile_ids`) as the reuse contract.
- [x] Map each requirement and scenario to its route, sub-check, catalog row,
      frontend render path and contract-test assertion.
- [x] Confirm the native readiness matrix (`run_matrix`/`run_profile_row`) and
      the DriftWatch adapter stay uninvoked, and record why.
- [x] Add `tests/forge_web_project_status_contract.rs` with the ten scenarios
      from design §7.

## 2. DFS — Requirement-by-requirement implementation

- [x] `src/api/status.rs`: add `CONTRACT_VERSION`,
      `ROUTE_PROJECT_STATUS`, `ROUTE_FLEET_STATUS`, the sub-check mapping, the
      overall-state reduction, `project_status` and `fleet_status`.
- [x] `src/api/workbench.rs`: widen `resolve`, `Resolved` and
      `redact_local_paths` to `pub(super)` (visibility only).
- [x] `src/api/mod.rs`: add `mod status;`, `Route::AdminProjectStatus { id }`
      and `Route::AdminFleetStatus`, the two router arms, the
      `required_permission` entries, the admin short-circuit entries and the
      unreachable-dispatch arms.
- [x] `src/api/admin.rs`: add the two `handle` arms delegating through
      `guarded`.
- [x] `src/api/command_catalog.rs`: add both routes to
      `IMPLEMENTED_WEB_ROUTES`; recatalogue `check` to `web`; repoint
      `fleet.status`; update the pinned web-row in-source test (count stays
      227).
- [x] `tests/forge_web_command_catalog_contract.rs`: extend the strict
      allowlist and add `check` to `web_ids`.
- [x] `frontend/index.html` + `frontend/app.js`: add the workbench status card
      and the dashboard fleet readiness tile, `textContent` only.
- [x] Implement the delta specs' scenarios alongside the code above (each
      scenario has a matching assertion in the contract test).

## 3. BFS — Cross-surface regression and completeness

- [x] Verify the workbench detail/plan/apply routes and the bearer `/v1`
      docto/checker routes are unchanged; the workbench, management, fleet and
      admin contracts stay green.
- [x] Verify no status/fleet response can carry the registered absolute path,
      a credential or an echoed hostile id; the journal, registry and project
      files are unchanged by any request (read-only).
- [x] Verify the catalog row count stays 227 and `readiness.*` rows stay
      `cli_only`; the `catalog_covers_every_clap_path` and pinned web-list
      tests pass.
- [x] Confirm no Core function, CLI dispatch or `API_CONTRACT_VERSION` was
      modified beyond the intended `status.rs` / `mod.rs` / `admin.rs` /
      `workbench.rs` visibility / `command_catalog.rs` / frontend / test
      allowlist surfaces.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] `cargo test` green for: `forge_web_project_status_contract` (new),
      `forge_web_command_catalog_contract` (updated),
      `forge_web_project_workbench_contract`,
      `forge_web_project_management_contract`, `forge_admin_api_contract`,
      `forge_portal_frontend_contract`, `portal_ui_contract`,
      `catalog_contract`, `cargo test --lib api::` and
      `cargo test --bin forge`. Record the actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and
      `openspec validate --all --strict --no-interactive` 0 failures with this
      change active.
- [x] `git diff --check` clean; review newly added files.
- [x] Archive with `openspec archive forge-web-project-status --yes` (no
      `--skip-specs`); promote the canonical specs; update HANDOFF with the
      evidence and no `current_spec` line.
