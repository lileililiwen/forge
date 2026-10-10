# Proposal: Web project-catalog browser

## Why

`forge project list|inspect|tags|languages|gaps` (contracts
`forge-project-catalog/0.1.0` + `forge-project-evidence/0.1.0`) and
`forge fleet inspect` are CLI-only: their six command-catalog rows sit
at `not_yet_web`. The SPA fleet table fetches
`GET /v1/projects/catalog` through six free-text predicate inputs and
constrains registry rows by the returned ids, but it never shows record
provenance (`source`, `source_revision`, `observed_at`, `freshness`),
tag/language distributions with counts, per-project all-source inspect,
evidence-backed gaps with verdicts, or a fleet-entry inspect. An
operator auditing "what does each source actually know" must drop to
the terminal. This is web UI/UX audit gap 3.

## What Changes

- Six `not_yet_web` rows become `web` with typed read-only admin GET
  routes (no writes, no provider calls, no shell):
  `fleet.inspect` → `GET /v1/admin/fleet/{entry}`;
  `project.list` → `GET /v1/admin/projects/catalog`;
  `project.inspect` → `GET /v1/admin/projects/{id}/catalog`;
  `project.tags` → `GET /v1/admin/projects/catalog/tags`;
  `project.languages` → `GET /v1/admin/projects/catalog/languages`;
  `project.gaps` → `GET /v1/admin/projects/catalog/gaps`.
- All six routes reuse the existing Core services only:
  `catalog::collect` + `CatalogQuery::from_pairs` + `apply` /
  `inspect_records` / `tag_counts` / `language_counts`,
  `gaps::build_report` with `GapFilters`, and
  `fleet::observe` + `inspect_entry`. Transports stay thin; no
  filtering/ordering/pagination rule is reimplemented.
- A read-only **Catalog browser** section on the existing projects view
  (`frontend/index.html` `#view-projects`, after the command reference,
  before the workbench — the gap-1 reference pattern): per-project
  inspect (all sources with provenance), tags/languages tables with
  counts, gaps list with category/verdict/evidence, fleet-entry inspect.
  Reuses `GET /v1/projects/catalog` where it already suffices and the
  six new admin routes where no route exists. No new frontend
  dependency; existing styles/a11y patterns reused.
- New contract tests: `tests/web_project_catalog_browser_contract.rs`
  (static frontend tokens + live admin-route round-trips).

## BFS Impact Map

- **Requirements/concepts:** `project-catalog-query-contract`
  (`forge-project-catalog/0.1.0`), `project-evidence-gap-assessment`
  (`forge-project-evidence/0.1.0`), `fleet-registry-observation`,
  `portal-web-ui` (standalone `frontend/` SPA).
- **Modules:** `src/api/model.rs` (6 `Route` variants),
  `src/api/router.rs` (route arms + permission + dispatch),
  `src/api/admin/routes.rs` (route constants),
  `src/api/admin/deploy.rs` or new `src/api/catalog_browser.rs`
  (guarded read handlers), `src/api/command_catalog/routes.rs`
  (`IMPLEMENTED_WEB_ROUTES` + route refs),
  `src/api/command_catalog/rows_fleet.rs` (6 rows `NotYetWeb` →
  `web_at`), `frontend/index.html` (one section),
  `frontend/app.js` (fetch + render + filters),
  `frontend/styles.css` (one additive block).
- **Contracts/callers:** `GET /v1/projects/catalog` family untouched;
  `GET /v1/admin/commands` envelope unchanged (6 rows change state,
  count stays 234 — conversion, not addition); `loadCommands`,
  workbench, portfolio, delivery untouched.
- **Persistence/integrations:** none. All reads open the registry
  read-only; catalog/gaps/fleet reads write no registry byte, table
  row or journal row; no provider, adapter, native toolchain, or shell.
- **Tests:** new `web_project_catalog_browser_contract`; existing
  `catalog_contract`, `catalog_cross_surface`, `project_gaps_contract`,
  `project_gaps_cross_surface`, `project_query_surface_contract`,
  `fleet_contract`, `forge_web_command_catalog_contract`,
  `portal_ui_contract`, `forge_web_navigation_contract`,
  `web_command_reference_browser_contract` must stay green.
- **Compatibility:** old API without the six routes → honest
  "unavailable" empty states; unknown project/entry → typed
  `unknown-project` / fleet refusal; invalid filter → typed
  `catalog-invalid`; 401 without session.
- **Concerns:** quality (success/failure/empty states mapped);
  security (session-gated, origin-checked, `textContent`-only rendering,
  no shell/path/command submission, credentials redacted by Core).

## Capabilities

- Operators can browse the normalized catalog with provenance from the
  dashboard without the terminal.
- Operators can inspect one project across all sources, see tag/language
  distributions, and read evidence-backed gaps with verdicts.
- Operators can inspect one fleet registry entry read-only.

## Non-goals

- `project github observe/propose/create` (provider-gated) — untouched.
- `fleet online` probe (live network/container probing) — untouched.
- `inventory show` (already `web`) — untouched.
- Any write: no register/import/new, no preview/confirm/apply, no
  journal row, no registry mutation.
- No new route beyond the six reads; no new view/route in `src/web.rs`;
  no sidebar change; no new frontend dependency.
