# Tasks: Web creation-catalog browser

## 1. BFS — Baseline and impact coverage

- [x] Confirm proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/api/model.rs` (`Route`, `err_status`), `src/api/router.rs` (pre-table chain, permission, short-circuit, exhaustiveness — note the 998/1000 cap), `src/api/admin/routes.rs`, `src/api/admin/deploy.rs` (`handle` dispatch + `deploy_id_gate`), `src/api/admin/agent_identity.rs` + `history.rs` (beside-handler matcher precedent), `src/api/command_catalog/` (rows_project, routes, `IMPLEMENTED_WEB_ROUTES`, catalog web vec), the six Core registries (`profile::`, `feature::`, `component::`, `ui_pattern::`, `standard::`, `procedure::`, `planner::`), `frontend/index.html` (`#view-projects`, `#command-reference`, `#catalog-browser`), `frontend/app.js` (`request`, `el()`, `setResultRole`, `catalogCommands`, `cmdrefRow`), `frontend/styles.css`, and the `portal-web-ui` canonical spec.
- [x] Map each requirement and scenario to its route constant, `Route` variant fields, handler, catalog-row conversion, HTML ids, app.js function, CSS class and contract-test assertion.
- [x] Record toolchain (Rust stable, Node for name preflight, Playwright via `tests/browser` for the live oracle) and ownership (backend thin reads + frontend section; no Core logic change).

## 2. DFS — Requirement-by-requirement implementation

- [x] Backend: new `src/api/admin/creation.rs` (`route_creation` matcher + guarded read-only handlers reusing Core only); 20 `ROUTE_ADMIN_CREATION_*` constants; 1 `Route::AdminCreation { registry, item, action }` variant; `router.rs` +1 chain line with same-line permission/short-circuit/exhaustiveness/`err_status` appends (999/1000 lines); 1 `deploy.rs` dispatch arm + 1 `handlers_core.rs` authorize arm; `IMPLEMENTED_WEB_ROUTES` +20 with route refs; 20 `NotYetWeb` rows → `web_at(Read, route, caps)` (count stays 234, `problems()` empty).
- [x] `frontend/index.html`: add the `#creation-catalog` section (heading, description, registry picker, search, list/inspect/resolve controls, standards project panel, intent panel, CLI-only remainder, live regions, error summary) inside `#view-projects` after `#catalog-browser`, before `#workbench`.
- [x] `frontend/app.js`: add the creation-catalog fetch/render/search group (`initCreationCatalog` + per-registry loaders), wired once at boot; explicit clicks only (search filters the cached list); `textContent`/`el()` only; `role="status"` results; honest unavailable/empty/error states; CLI-only remainder derived from `catalogCommands`; no new endpoint string beyond the twenty routes; no `eval`/`Function`/`innerHTML`/shell.
- [x] `frontend/styles.css`: one additive block for the creation-catalog section at the 12px floor, reusing panel/badge/result tokens.
- [x] `tests/web_creation_catalog_browser_contract.rs`: 10 tests — static-token oracle (section/controls ids, route-string fragments, no-shell/no-write boundary) plus live admin-route round-trips (six lists, inspects incl. unknown → typed + standard pack selector from the live list, four resolves incl. missing-subject + unknown, intent validate happy + malformed + journal-free, standard check/diff + intent plans on a registered fixture project incl. scrub asserts, hostile ids, unmanaged project, 401s, catalog pins for all 20 rows + 5 CLI-only leftovers). All green.
- [x] Implement the delta spec's scenarios alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify CLI/MCP creation surfaces bytes unchanged (profile/feature/component/ui-pattern/standard/procedure/intent suites green: 15+5+10+12+7+16+8+9+9+3+33+5 passed).
- [x] Verify catalog count stays 234 with 20 conversions and `problems()` empty (`catalog_has_no_integrity_problems`, `web_rows_only_point_at_implemented_routes` green).
- [x] Verify no write path added (grep: no new POST, no `run_with_operation`/`record_operation` call in the new module — only a doc comment naming the deliberate omission; registry bytes identical before/after reads asserted in-contract).
- [x] Verify existing suites green: `forge_web_command_catalog_contract` 8 + pre-existing `cap` failure (byte-identical signature to the recorded pristine failure; no CLI file in this diff), `portal_ui_contract` 21, `forge_web_navigation_contract` 9, `web_command_reference_browser_contract` 6, `forge_web_maintainer_surface_contract` 5, project-catalog/history/identity browser contracts 10/9/10, `forge_web_project_workbench_contract` 10 + pre-existing `innerHTML` failure (hunk-excluded by diff: workbench region untouched).
- [x] Verify a11y contracts: native controls, live regions, error-summary focus, keyboard operability, 12px floor, no new motion, `textContent`-only.
- [x] Verify no new frontend dependency and no `src/web.rs`/sidebar/router-allowlist change.
- [x] Verify `router.rs` at 999/1000 lines post-`cargo fmt` (beside-handler matcher + `route_beside` fold + `creation_error_status` kept the table untouched) and every touched Rust file ≤1000 lines (gate `source-file-size` 402/402 pass).
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (pre-existing `ShareSurface` warning only).
- [x] `cargo test` green for: new `web_creation_catalog_browser_contract` (10/10) plus the regression list in §3; `cargo test --lib` 1213/0. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS, `node scripts/check-spec-governance.mjs` PASS, and `openspec validate --all --strict --no-interactive` 100/0 with this change active.
- [x] `git diff --check` clean; review newly added files too.
- [x] Live oracle (throwaway API:8766+web:4173 default ports, scratch `FORGE_REGISTRY` with alpha fixture, `FORGE_ADMIN_PROJECTS_ROOT` set, real Chromium via bundled Playwright, script in `/tmp` — never committed): login → `/projects` → list 6 profiles → inspect rust-web → resolve audit-action → validate intent hash → check absent → plans empty → CLI-only badges; curl cross-check of all twenty routes (19×200 + typed 400 diff-without-snapshot) + anon 401; **zero JS console/page/network errors, zero failed requests**.
- [x] `forge gate --dry-run` rehearse, then full `forge gate --timeout-secs 600`; verdict **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (402/402 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure.
- [ ] Archive with `openspec archive web-creation-catalog-browser --yes` (no `--skip-specs`); promote the canonical spec; verify the canonical Purpose is source-backed (repair immediately if the archiver stamped TBD, before committing); update HANDOFF with evidence and pointer handling.
