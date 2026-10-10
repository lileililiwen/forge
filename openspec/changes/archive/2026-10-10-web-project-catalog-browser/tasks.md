# Tasks: Web project-catalog browser

## 1. BFS — Baseline and impact coverage

- [x] Confirm proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/api/model.rs` (`Route`), `src/api/router.rs` (route arms, permission, dispatch), `src/api/admin/routes.rs`, `src/api/admin/deploy.rs` (guarded pattern), `src/api/handlers_core.rs` (`handle_catalog_query`), `src/api/server.rs` (catalog params), `src/catalog/` (collect/apply/inspect/tag/language), `src/doctor/gaps/` (build_report/filters), `src/fleet/` (observe/inspect_entry), `src/api/command_catalog/` (rows_fleet, routes, IMPLEMENTED_WEB_ROUTES), `frontend/index.html` (`#view-projects`, `#command-reference`), `frontend/app.js` (`request`, `fleetFilterParams`, `loadCommands`, `el()`, `setResultRole`), `frontend/styles.css` (panel/badge/result tokens), and the `portal-web-ui` + catalog + gaps canonical specs.
- [x] Map each requirement and scenario to its route constant, `Route` variant, handler, catalog-row conversion, HTML ids, app.js function, CSS class and contract-test assertion.
- [x] Record toolchain (Rust stable, Node for name preflight, Playwright via `tests/browser` for the live oracle) and ownership (backend thin reads + frontend section; no Core logic change).

## 2. DFS — Requirement-by-requirement implementation

- [x] Backend routes: add 6 constants (`ROUTE_ADMIN_CATALOG*`, `ROUTE_ADMIN_FLEET_INSPECT`), 6 `Route` variants, router arms (literals before `{id}`) + OPTIONS + permission (`None`) + dispatch, guarded read-only handlers reusing Core only (no shell/provider/write/journal), `IMPLEMENTED_WEB_ROUTES` + route refs, and convert the 6 `NotYetWeb` rows to `web_at(Read, route, caps_registry)`.
- [x] `frontend/index.html`: add the `#catalog-browser` section (heading, description, project inspect controls, tags/languages tables, gaps controls+list, fleet-entry inspect, live regions, error summaries) inside `#view-projects` after `#command-reference`, before `#workbench`.
- [x] `frontend/app.js`: add the catalog-browser fetch/render/filter group (`initCatalogBrowser` + per-resource loaders), wired once at boot; `textContent`/`el()` only; `role="status"` results; honest unavailable/empty/error states; no new endpoint string beyond the six routes; no `eval`/`Function`/`innerHTML`/shell.
- [x] `frontend/styles.css`: one additive block for the browser section/tables/controls at the 12px floor, reusing panel/badge/result tokens.
- [x] `tests/web_project_catalog_browser_contract.rs`: static-token oracle (section/controls ids, six route strings, provenance/tags/gaps/evidence rendering, no-shell/no-write boundary) plus live admin-route round-trips (list/inspect/tags/languages/gaps/fleet-inspect happy paths, unknown id, invalid filter, 401, read-only registry bytes).
- [x] Implement the delta spec's scenarios alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify `GET /v1/projects/catalog` family bytes unchanged (CLI/MCP/API parity suites green).
- [x] Verify catalog count stays 234 with 6 conversions and `problems()` empty.
- [x] Verify no write path added (grep: no new POST, no `run_with_operation`, no journal write in the new module; registry bytes identical before/after reads).
- [x] Verify existing suites green: `catalog_contract`, `catalog_cross_surface`, `project_gaps_contract`, `project_gaps_cross_surface`, `project_query_surface_contract`, `fleet_contract`, `forge_web_command_catalog_contract`, `portal_ui_contract`, `forge_web_navigation_contract`, `web_command_reference_browser_contract`.
- [x] Verify a11y contracts: native controls, live regions, error-summary focus, keyboard operability, 12px floor, no new motion, `textContent`-only.
- [x] Verify no new frontend dependency and no `src/web.rs`/sidebar/router-allowlist change.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (no new warnings beyond the 3 pre-existing).
- [x] `cargo test` green for: new `web_project_catalog_browser_contract` plus the regression list in §3. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files too.
- [x] Live oracle (throwaway API+web, scratch `FORGE_REGISTRY`, real Chromium via bundled Playwright, script in `/tmp` — never committed): catalog browser renders inspect/tags/languages/gaps/fleet-inspect with zero JS console errors. Record evidence.
- [x] `forge gate --dry-run` rehearse, then full `forge gate` bounded (e.g. `--timeout-secs 600`); verdict recorded; no new failure attributable to this change (known pre-existing `project-runtime` timeout blocks).
- [ ] Archive with `openspec archive web-project-catalog-browser --yes` (no `--skip-specs`); promote the canonical spec; update HANDOFF with evidence and pointer handling.
