# Design: Web assurance browser

## 1. Ownership and placement

The browser is a section of the **projects view**, not a new view — the
gap-6 pattern (`web-creation-catalog-browser` design §1).
`src/web.rs::asset_name` stays an exact allowlist; no route, sidebar,
`VIEW_BY_PATH`/`VIEW_IDS`/`VIEW_CRUMBS` or navigation-contract change.
Placement: `<section id="assurance-browser">` inside `#view-projects`,
after `#creation-catalog`, before `#workbench`, so every signed-in
operator lands on it and it never interferes with the workbench,
management, portfolio or delivery views.

Owner: backend `src/api/` (thin read handlers + wiring) and `frontend/`
assets. No CLI/MCP/registry/journal/Core-logic change.

## 2. Route contracts (all GET, session-gated, read-only)

Constants live in `src/api/admin/routes.rs` and are re-exported through
`command_catalog/routes.rs` so the catalog and the live router stay in
lockstep:

| # | Constant | Method + path |
|---|---|---|
| 1 | `ROUTE_ADMIN_CONTRACTS` | `GET /v1/admin/contracts` |
| 2 | `ROUTE_ADMIN_CONTRACT_INSPECT` | `GET /v1/admin/contracts/{family}` |
| 3 | `ROUTE_ADMIN_SPECS` | `GET /v1/admin/projects/{id}/specs` |
| 4 | `ROUTE_ADMIN_SPEC_INSPECT` | `GET /v1/admin/projects/{id}/specs/{spec}` |
| 5 | `ROUTE_ADMIN_SPEC_ROUTE` | `GET /v1/admin/projects/{id}/spec/route?finding=` |
| 6 | `ROUTE_ADMIN_REMEDIATE_SCAN` | `GET /v1/admin/projects/{id}/remediate/scan` |
| 7 | `ROUTE_ADMIN_REMEDIATE_DIFF` | `GET /v1/admin/projects/{id}/remediate/diff?finding=&pack=` |
| 8 | `ROUTE_ADMIN_DESCRIBE_LIST` | `GET /v1/admin/projects/{id}/describe/proposals` |
| 9 | `ROUTE_ADMIN_DESCRIBE_SHOW` | `GET /v1/admin/projects/{id}/describe/proposals/{proposal}` |
| 10 | `ROUTE_ADMIN_CLASSIFY_LIST` | `GET /v1/admin/projects/{id}/classify/proposals` |
| 11 | `ROUTE_ADMIN_CLASSIFY_SHOW` | `GET /v1/admin/projects/{id}/classify/proposals/{proposal}` |
| 12 | `ROUTE_ADMIN_CONTRACT_EMIT` | `GET /v1/admin/projects/{id}/contracts/emit?family=` |
| 13 | `ROUTE_ADMIN_GOVERNANCE_LIST` | `GET /v1/admin/projects/{id}/governance` |
| 14 | `ROUTE_ADMIN_GOVERNANCE_STATUS` | `GET /v1/admin/projects/{id}/governance/status` |
| 15 | `ROUTE_ADMIN_GOVERNANCE_INSPECT` | `GET /v1/admin/projects/{id}/governance/inspect` |
| 16 | `ROUTE_ADMIN_ANALYTICS_METRICS` | `GET /v1/admin/projects/{id}/analytics/metrics?window_days=` |
| 17 | `ROUTE_ADMIN_STUDIO_PREVIEW` | `GET /v1/admin/projects/{id}/studio/preview` |

Handler module: new `src/api/admin/assurance.rs` with a `route_assurance`
matcher tried before the main table via the existing `route_beside`
fold (creation precedent), so `router.rs` keeps a constant line count
(999/1000). The nine assurance sections ride the shared
`Route::AdminCreation { registry, item, action }` validated-key triple —
no new variant, no new table/permission/authorize/exhaustiveness arm:
section keys (`contracts`, `specs`, `spec`, `remediate`, `describe`,
`classify`, `governance`, `analytics`, `studio`) are disjoint from every
creation registry key, and one `dispatch_creation` branch in
`src/api/admin/deploy.rs` (constant line count) sends assurance sections
to `assurance::dispatch` and every other registry to
`creation::dispatch`; unknown triples never match (404, no echo).

Guards on every path: `guarded` (session + exact origin) → `deploy_id_gate`
(project-bound) or pure (contracts list/inspect, no id) → per-segment
gates (`creation_id_gate`-style kebab gate for spec/proposal/family ids;
`finding`/`pack`/`family`/`window_days` query gates: non-empty ≤256 chars,
`pack` must contain `@` when present, `window_days` must parse `u32`,
family must be in `supported_families()`); hostile ids → static typed
`400` without echo; unmanaged ids → typed `404`; traversal adding a
segment matches no route → `404`. Every response carries
`contract: API_CONTRACT_VERSION` plus the registry contract where one
exists; absolute project paths are scrubbed (`scrub_*`); `project_path`
is projected out of agent-adjacent shapes where present.

Core reuse (no new logic, no write):

- `spec::list_specs` / `spec::read_spec` (prefix match mirrors
  `parse_spec_id`); `spec::route_finding` with a CLI-parity source:
  `driftwatch-` prefix → policy finding, else a doctor-fail input for the
  named id. No `apply_routing`.
- `remediation::scan`; `remediation::build_plan(dir, finding, pack)` +
  `remediation::diff(&plan)`. The `--plan` file path is not reproduced.
- `semantic::decide::list|read` for both describe and classify (kinds
  share the store; the UI labels the kind).
- `contract::load_manifest` + `CONTRACTS` (list);
  `family_schema_path` + vendored schema `required` array (inspect);
  emit dispatches `emit_gate_result` / `emit_release_evidence` /
  `emit_readiness` over the server-resolved dir; readiness empty →
  typed `unavailable` (not 500); unknown family → typed `400`.
- `governance::list_providers`; `governance::evaluate_project` only
  (never `check_project`); external enabled provider → honest
  `unavailable-with-reason` naming the provider, never an adapter run.
- `analytics::aggregate_project_metrics(&registry, Some(DoctorSummary::
  default()), &MetricsAggregateOptions { default_window_days,
  external_observations: vec![] })`; window defaults to
  `DEFAULT_WINDOW_DAYS`, clamped to `[MIN_WINDOW_DAYS,
  MAX_WINDOW_DAYS]` with typed `400` outside parse; no
  `inspect_external_planes`, no `save_metrics_summary`, no
  `record_operation`.
- `studio::load_session` + `studio::envelope_from_session`; `None` →
  `PreviewEnvelope::from_session(id, "r0",
  &SessionPreviewState::default())` (`state: none`), mirroring the CLI
  status path; never `start_preview`/`stop_preview`/spawn.

Failures map beside the handlers in an `assurance_error_status` helper
(kept out of `err_status` for the line cap): reachable user-error codes
→ `400`, unknown ids → `404`, external/unavailable → `200` with
`unavailable + reason` (honest, not an error), server faults → `500`
without path/secret material.

## 3. Catalog conversions

In `rows_assurance.rs`: `spec list|inspect|route`, `remediate scan|diff`,
`describe list|show`, `classify list|show`, `contract list|inspect|emit`,
`governance list|status|inspect` convert `NotYetWeb` → `web_at(Read,
route, caps)` with caps verbatim. In `rows_platform.rs`:
`analytics metrics`, `studio preview` convert the same way
(`studio preview` keeps its `LocalWrite` risk — the verb owns start/stop
— but the web route exposes only the status read; the design documents
the restriction and the UI never offers start/stop). Count stays 234
(conversion); `problems()` empty. `IMPLEMENTED_WEB_ROUTES` +17 with route
refs; `web_rows_only_point_at_implemented_routes` stays green.

Left CLI-only or already-web, untouched: `spec generate|apply`,
`remediate plan|apply`, `classify apply|approve|reject`,
`studio spec|refine` (already web POSTs), `describe suggest|approve|
reject`, `classify suggest|derive`, `contract validate` (file/stdin),
`governance use`, `analytics inspect` (provider).

## 4. Frontend

One `<section id="assurance-browser">` after `#creation-catalog`: heading,
description, project picker (reuses `#workbench-project` value when
present, else its own input), registry picker (8 options), per-registry
controls (list button; inspect/spec/proposal/family/finding inputs;
route finding input; diff finding+pack inputs; emit family picker;
metrics window input; governance status/inspect buttons; preview read
button), cached client-side search over the last list, results in
`role="status"`, error-summary focus, native controls, `textContent` /
`el()` only, no `innerHTML`, no new dependency. CLI-only remainder
(`describe suggest|approve|reject`, `classify suggest|derive`, `contract
validate`, `governance use`, `analytics inspect`, `studio spec|refine`
plus the already-web writes) renders as availability badges with reasons
derived from the loaded command catalog. Explicit reads only (no
auto-load); every fetch carries the session cookie via the existing
`request` helper.

## 5. Tests

New `tests/web_assurance_browser_contract.rs` (≈12 tests): static-token
oracle (section/controls ids, route-string fragments, no-shell/no-write
boundary) + live admin-route round-trips over a scratch registry fixture
(seventeen lists/inspects incl. unknown → typed, hostile → 400,
unmanaged → 404, anon → 401; scan empty-findings shape; diff rebuild
parity with CLI `build_plan|diff`; governance local pass + external
unavailable; analytics default + clamp + registry-bytes-identical;
studio none-envelope; scrub asserts — no absolute path, no
`project_path`; catalog pins for all 17 rows + CLI-only leftovers).
Pin updates in `catalog.rs` (web vec +17) and
`tests/forge_web_command_catalog_contract.rs` (web-route + web-id
allowlists +17).

## 6. Migration / rollout

No migration: additive GETs + one SPA section. No Core, CLI, MCP, portal,
journal, or registry-schema change. Rollback = revert the one change.
