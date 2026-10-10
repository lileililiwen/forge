# Tasks: Web assurance browser

## 1. BFS — Baseline and impact coverage

- [x] Confirm proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/api/model.rs` (`Route`, `err_status`), `src/api/router.rs` (pre-table chain, permission, short-circuit, exhaustiveness — note the 999/1000 cap), `src/api/admin/routes.rs`, `src/api/admin/deploy.rs` (`handle` dispatch + `deploy_id_gate`), `src/api/admin/creation.rs` + `agent_identity.rs` + `history.rs` (beside-handler matcher precedent + `route_beside` fold), `src/api/command_catalog/` (rows_assurance, rows_platform, routes, `IMPLEMENTED_WEB_ROUTES`, catalog web vec), the eight Core surfaces (`spec::list_specs|read_spec|route_finding`, `remediation::scan|build_plan|diff`, `semantic::decide::list|read`, `contract::load_manifest|family_schema_path|supported_families|emit_*`, `governance::list_providers|evaluate_project`, `analytics::aggregate_project_metrics`, `studio::load_session|envelope_from_session`), `frontend/index.html` (`#view-projects`, `#creation-catalog`), `frontend/app.js` (`request`, `el()`, `setResultRole`, `catalogCommands`, `cmdrefRow`), `frontend/styles.css`, and the `portal-web-ui` canonical spec.
- [x] Map each requirement and scenario to its route constant, `Route` variant fields, handler, catalog-row conversion, HTML ids, app.js function, CSS class and contract-test assertion.
- [x] Record toolchain (Rust stable, Node for name preflight, Playwright via `tests/browser` for the live oracle) and ownership (backend thin reads + frontend section; no Core logic change).

## 2. DFS — Requirement-by-requirement implementation

- [x] Backend: new `src/api/admin/assurance.rs` (`route_assurance` matcher + guarded read-only handlers reusing Core only); 17 `ROUTE_ADMIN_*` constants; 1 `Route::AdminAssurance { section, item, action }` variant; `router.rs` constant (999/1000, shared triple, no new variant/arm); 1 `deploy.rs` `dispatch_creation` branch (constant line count, no new arm); `IMPLEMENTED_WEB_ROUTES` +17 with route refs; 17 `NotYetWeb` rows → `web_at(Read, route, caps)` (count stays 234, `problems()` empty).
- [x] `frontend/index.html`: add the `#assurance-browser` section (heading, description, project input, registry picker, per-registry controls, CLI-only remainder, live regions, error summary) inside `#view-projects` after `#creation-catalog`, before `#workbench`.
- [x] `frontend/app.js`: add the assurance fetch/render/search group (`initAssuranceBrowser` + per-registry loaders), wired once at boot; explicit clicks only (search filters the cached list); `textContent`/`el()` only; `role="status"` results; honest unavailable/empty/error states; CLI-only remainder derived from `catalogCommands`; no new endpoint string beyond the seventeen routes; no `eval`/`Function`/`innerHTML`/shell.
- [x] `frontend/styles.css`: one additive block for the assurance section at the 12px floor, reusing panel/badge/result tokens.
- [x] `tests/web_assurance_browser_contract.rs`: ≈12 tests — static-token oracle (section/controls ids, route-string fragments, no-shell/no-write boundary) plus live admin-route round-trips (seventeen lists/inspects incl. unknown → typed + hostile → 400 + unmanaged → 404 + anon → 401, scan shape, diff rebuild parity, governance local + external-unavailable, analytics default + clamp + no-write, studio none-envelope, scrub asserts, catalog pins for all 17 rows + CLI-only leftovers). All green.
- [x] Implement the delta spec's scenarios alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify CLI/MCP assurance surfaces bytes unchanged (spec/remediate/semantic/contract/governance/analytics/studio suites green; record counts).
- [x] Verify catalog count stays 234 with 17 conversions and `problems()` empty (`catalog_has_no_integrity_problems`, `web_rows_only_point_at_implemented_routes` green).
- [x] Verify no write path added (grep: no new POST, no `record_operation`/`save_*`/`check_project`/`inspect_external_planes`/`start_preview`/`stop_preview`/`spawn` call in the new module; registry bytes identical before/after reads asserted in-contract).
- [x] Verify existing suites green: `forge_web_command_catalog_contract` (record pass/fail vs pristine), `portal_ui_contract`, `forge_web_navigation_contract`, `web_command_reference_browser_contract`, `forge_web_maintainer_surface_contract`, creation-catalog/history/identity/project-catalog browser contracts, `forge_web_project_workbench_contract` (record pre-existing failures byte-identical via `git stash -u` rerun where needed).
- [x] Verify a11y contracts: native controls, live regions, error-summary focus, keyboard operability, 12px floor, no new motion, `textContent`-only.
- [x] Verify no new frontend dependency and no `src/web.rs`/sidebar/router-allowlist change.
- [x] Verify `router.rs` ≤1000 lines post-`cargo fmt` and every touched Rust file ≤1000 lines.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (record warnings).
- [x] `cargo test` green for: new `web_assurance_browser_contract` plus the regression list in §3; `cargo test --lib`. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS, `node scripts/check-spec-governance.mjs` PASS, and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files too.
- [x] Live oracle (throwaway API+web on default ports, scratch `FORGE_REGISTRY` with fixture, `FORGE_ADMIN_PROJECTS_ROOT` set, real Chromium via bundled Playwright, script in `/tmp` — never committed): login → `/projects` → per-registry list/inspect/route-or-scan-or-diff-or-emit-or-metrics-or-preview rendering + CLI-only badges; curl cross-check of all seventeen routes + anon 401; **zero JS console/page/network errors, zero failed requests**.
- [x] `forge gate --dry-run` rehearse, then full `forge gate` with bounded timeout (e.g. `--timeout-secs 600`); record verdict with attribution (known pre-existing project-runtime 60s timeout; attributable blocks).
- [ ] Archive with `openspec archive web-assurance-browser --yes` (no `--skip-specs`); promote the canonical spec; verify the canonical Purpose is source-backed (repair immediately if the archiver stamped TBD, before committing); update HANDOFF with evidence and pointer handling.
