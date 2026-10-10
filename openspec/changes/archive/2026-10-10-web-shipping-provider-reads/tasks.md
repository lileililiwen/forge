# Tasks: Web shipping provider reads

## 1. BFS — Baseline and impact coverage

- [x] Confirm proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/api/model.rs` (`Route`, `err_status`), `src/api/router.rs` (pre-table chain, permission, short-circuit, exhaustiveness — note the 999/1000 cap), `src/api/admin/routes.rs`, `src/api/admin/deploy.rs` (`handle` dispatch + `deploy_id_gate`), `src/api/admin/creation.rs` + `agent_identity.rs` + `history.rs` + `assurance.rs` (beside-handler matcher precedent + `route_beside` fold + `dispatch_creation`), `src/api/command_catalog/` (rows_shipping, routes, `IMPLEMENTED_WEB_ROUTES`, catalog web vec), the Core surfaces (`publish::providers::load_config`, `plugins::resolve_config_path|load_descriptors|list`, `provider::matrix|inspect`), `frontend/index.html` (`#delivery`, history card), `frontend/app.js` (`request`, `el()`, `setResultRole`, `catalogCommands`), `frontend/styles.css`, and the `portal-web-ui` canonical spec.
- [x] Map each requirement and scenario to its route constant, `Route` key triple, handler, catalog-row conversion, HTML ids, app.js function, CSS class and contract-test assertion.
- [x] Record toolchain (Rust stable, Node for name preflight, Playwright via `tests/browser` for the live oracle) and ownership (backend thin reads + frontend card; no Core logic change).

## 2. DFS — Requirement-by-requirement implementation

- [x] Backend: new `src/api/admin/shipping_reads.rs` (`route_shipping_reads` matcher + guarded read-only handlers reusing Core only); 5 `ROUTE_ADMIN_*` constants; 0 new `Route` variants (shared `AdminCreation` triple); `router.rs` untouched (constant 999/1000); `assurance.rs` +1 dispatch branch; `admin/mod.rs` +1 fold line; `IMPLEMENTED_WEB_ROUTES` +5 with route refs; 5 `NotYetWeb` rows → `web_at(Read, route, caps)` (count stays 234, `problems()` empty).
- [x] `frontend/index.html`: add the shipping-reads card (heading, description, project/provider inputs, five loader buttons, CLI-only remainder, live regions, error summary) inside `#delivery` after the history card, before `#delivery-actions-title`.
- [x] `frontend/app.js`: add the shipping-reads fetch/render group (`initShippingReads` + five loaders), wired once at boot; explicit clicks only; `textContent`/`el()` only; `role="status"` results; honest unavailable/empty/error states; CLI-only remainder derived from `catalogCommands`; no new endpoint string beyond the five routes; no `eval`/`Function`/`innerHTML`/shell.
- [x] `frontend/styles.css`: one additive block for the shipping-reads card at the 12px floor, reusing panel/badge/result tokens.
- [x] `tests/web_shipping_provider_reads_contract.rs`: ≈10 tests — static-token oracle (section/controls ids, route-string fragments, no-shell/no-write/no-probe boundary) plus live admin-route round-trips (five lists/inspects incl. unknown → typed + hostile → 400 + unmanaged → 404 + anon → 401, matrix not-run parity, plugins missing-config empty registry, scrub asserts, catalog pins for all 5 rows + Non-goals leftovers). All green.
- [x] Implement the delta spec's scenarios alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify CLI/MCP shipping surfaces bytes unchanged (publish/provider/plugins suites green; record counts).
- [x] Verify catalog count stays 234 with 5 conversions and `problems()` empty (`catalog_has_no_integrity_problems`, `web_rows_only_point_at_implemented_routes` green).
- [x] Verify no write/probe path added (grep: no new POST, no `record_operation`/`invoke_provider`/`run_provider`/`run_controlled`/`probe_`/`enable|disable` call in the new module; registry bytes identical before/after reads asserted in-contract).
- [x] Verify existing suites green: `forge_web_command_catalog_contract` (record pass/fail vs pristine), `portal_ui_contract`, `forge_web_navigation_contract`, `web_command_reference_browser_contract`, creation/assurance/history/identity/catalog browser contracts, `forge_web_project_workbench_contract` (record pre-existing failures byte-identical via `git stash -u` rerun where needed).
- [x] Verify a11y contracts: native controls, live regions, error-summary focus, keyboard operability, 12px floor, no new motion, `textContent`-only.
- [x] Verify no new frontend dependency and no `src/web.rs`/sidebar/router-allowlist change.
- [x] Verify `router.rs` ≤1000 lines post-`cargo fmt` and every touched Rust file ≤1000 lines.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (record warnings).
- [x] `cargo test` green for: new `web_shipping_provider_reads_contract` plus the regression list in §3; `cargo test --lib`. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS, `node scripts/check-spec-governance.mjs` PASS, and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files too.
- [x] Live oracle (throwaway API+web on default ports, scratch `FORGE_REGISTRY` with fixture, `FORGE_ADMIN_PROJECTS_ROOT` set, real Chromium via bundled Playwright, script in `/tmp` — never committed): login → `/delivery` → five reads rendering + CLI-only badges; curl cross-check of all five routes + anon 401; **zero JS console/page/network errors, zero failed requests**.
- [x] `forge gate --dry-run` rehearse, then full `forge gate` with bounded timeout (e.g. `--timeout-secs 600`); record verdict with attribution (known pre-existing project-runtime 60s timeout; attributable blocks).
- [ ] Archive with `openspec archive web-shipping-provider-reads --yes` (no `--skip-specs`); promote the canonical spec; verify the canonical Purpose is source-backed (repair immediately if the archiver stamped TBD, before committing); update HANDOFF with evidence and pointer handling.
