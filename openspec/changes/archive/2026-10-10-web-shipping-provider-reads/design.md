# Design: Web shipping provider reads

## 1. Ownership and placement

The reads panel is a card of the **delivery view**, not a new view — the
gap-7 pattern (`web-assurance-browser` design §1). `src/web.rs::
asset_name` stays an exact allowlist; no route, sidebar,
`VIEW_BY_PATH`/`VIEW_IDS`/`VIEW_CRUMBS` or navigation-contract change.
Placement: `<section class="wb-card" aria-labelledby=
"shipping-reads-title">` inside `#delivery`, after the history card
(`#history-status-result` block), before `#delivery-actions-title`, so
it sits beside the delivery reads it audits and never interferes with
the workbench, management, portfolio or projects views.

Owner: backend `src/api/` (thin read handlers + wiring) and `frontend/`
assets. No CLI/MCP/registry/journal/Core-logic change.

## 2. Route contracts (all GET, session-gated, read-only)

Constants live in `src/api/admin/routes.rs` and are re-exported through
`command_catalog/routes.rs` so the catalog and the live router stay in
lockstep:

| # | Constant | Method + path |
|---|---|---|
| 1 | `ROUTE_ADMIN_PUBLISH_PROVIDERS` | `GET /v1/admin/projects/{id}/publish/providers` |
| 2 | `ROUTE_ADMIN_PUBLISH_PROVIDER_INSPECT` | `GET /v1/admin/projects/{id}/publish/providers/{provider}` |
| 3 | `ROUTE_ADMIN_EVIDENCE_MATRIX` | `GET /v1/admin/providers/matrix` |
| 4 | `ROUTE_ADMIN_EVIDENCE_PROVIDER_INSPECT` | `GET /v1/admin/providers/{provider}` |
| 5 | `ROUTE_ADMIN_SHIPPING_PLUGINS` | `GET /v1/admin/projects/{id}/plugins` |

Handler module: new `src/api/admin/shipping_reads.rs` with a
`route_shipping_reads` matcher tried via the existing `route_beside`
fold (creation/assurance precedent), so `router.rs` keeps a constant
line count (999/1000). The three new sections ride the shared
`Route::AdminCreation { registry, item, action }` validated-key triple —
no new variant, no new table/permission/authorize/exhaustiveness arm:
section keys (`shipping-providers`, `evidence-providers`,
`shipping-plugins`) are disjoint from every creation/assurance key, and
`assurance::dispatch_creation` gains one three-way branch sending
shipping sections to `shipping_reads::dispatch` (constant-line-count
`deploy.rs` untouched); unknown triples never match (404, no echo).

Matcher shapes (all `GET`; `OPTIONS` preflight beside the GETs):

- `["v1","admin","projects",id,"publish","providers"]` →
  `("shipping-providers", id, "list")`
- `["v1","admin","projects",id,"publish","providers",provider]` →
  `("shipping-providers", "{id}/{provider}", "inspect")`
- `["v1","admin","providers","matrix"]` →
  `("evidence-providers", "", "matrix")`
- `["v1","admin","providers",provider]` (literal `matrix` claimed by the
  arm above, so no collision) →
  `("evidence-providers", provider, "inspect")`
- `["v1","admin","projects",id,"plugins"]` →
  `("shipping-plugins", id, "list")`

Guards on every path: `guarded` (session + exact origin) →
`deploy_id_gate` (project-bound) or pure (matrix + provider inspect, no
id) → per-segment gates (assurance-style kebab gate for provider ids;
hostile → static typed `400` without echo; unmanaged project → typed
`404`; traversal adding a segment matches no route → `404`). Every
response carries `contract: API_CONTRACT_VERSION` plus the relevant
registry contract where one exists; absolute project paths are scrubbed
(`scrub_*`); provider `command` paths are projected through the scrub
so no absolute server path leaks.

Core reuse (no new logic, no write, no journal):

- Publish providers: `publish::providers::load_config` over the
  server-resolved config path (`FORGE_PUBLISH_PROVIDER_CONFIG` env
  override else `<project_dir>/.forge/providers.yaml`, mirroring the
  publish plan handler's resolution without its provider/revision
  requirements). Missing file → `200` honest `unavailable-with-reason`
  (never the absolute path). List serializes `ProviderConfig`
  unchanged; inspect finds one entry by id, unknown → typed `404`.
- Evidence providers: `provider::matrix(false)` — the browser never
  passes `live`, so every row is `not-run` and no binary is probed; no
  `record_operation` (the CLI's journal line is deliberately not
  reproduced, journal-free like assurance). `provider::inspect(id)`
  static descriptor — no probe, no contact; unknown id → the Core typed
  error mapping.
- Plugins: `providers::load_config` + `plugins::load_descriptors` +
  `plugins::list` over the same server-resolved path. Missing file →
  honest empty-registry `200` (`plugins: []`, mirroring the CLI's
  missing-config answer). Records serialize the CLI's JSON projection
  (`id`, `kind`, `enabled`, `state`, `reason`, `command` scrubbed,
  `capabilities`, `description`).

Failures map beside the handlers in a `shipping_error_status` helper
(kept out of `err_status` for the line cap): reachable user-error codes
→ `400`, unknown provider/entry → `404`, missing/unconfigured →
`200` with `unavailable + reason` (honest, not an error), server faults
→ `500` without path/secret material.

## 3. Catalog conversions

In `rows_shipping.rs`: `publish.provider list|inspect`,
`provider matrix|inspect`, `plugins list` convert `NotYetWeb` →
`web_at(Read, route, caps)` with caps verbatim (`none` — same as today).
Count stays 234 (conversion); `problems()` empty.
`IMPLEMENTED_WEB_ROUTES` +5 with route refs;
`web_rows_only_point_at_implemented_routes` stays green.

Left CLI-only / provider-required / capability-gated, untouched:
`publish sync|db|fleet|prepare|deploy|all`, `docs translate`,
`provider run`, `fleet online`, `project.github *`, `push`/`mirror`,
`readiness *`, `deploy observe`, transports, agent/identity writes, and
`publish provider enable|disable` (stays `not_yet_web`).

## 4. Frontend

One `<section class="wb-card" aria-labelledby="shipping-reads-title">`
after the history card in `#delivery`: heading, description, project
input (reuses `#delivery-project` value when present, else its own
input), provider-id input, five explicit loader buttons (publish
providers list, publish provider inspect, evidence matrix, evidence
provider inspect, plugins list), results in `role="status"`,
error-summary focus, native controls, `textContent` / `el()` only, no
`innerHTML`, no new dependency. CLI-only remainder (the Non-goals verbs
above) renders as availability badges with reasons derived from the
loaded command catalog, plus a pointer to the existing
`catalog-fleet-inspect` control for `fleet inspect`. Explicit reads only
(no auto-load); every fetch carries the session cookie via the existing
`request` helper.

## 5. Tests

New `tests/web_shipping_provider_reads_contract.rs` (≈10 tests):
static-token oracle (section/controls ids, route-string fragments,
no-shell/no-write/no-probe boundary) + live admin-route round-trips
over a scratch registry fixture (five lists/inspects incl. unknown →
typed, hostile → 400, unmanaged → 404, anon → 401; matrix not-run
parity with `provider::matrix(false)`; plugins missing-config empty
registry; scrub asserts — no absolute path; catalog pins for all 5 rows
+ Non-goals leftovers). Pin updates in `catalog.rs` (web vec +5) and
`tests/forge_web_command_catalog_contract.rs` (web-route + web-id
allowlists +5).

## 6. Migration / rollout

No migration: additive GETs + one SPA card. No Core, CLI, MCP, portal,
journal, or registry-schema change. Rollback = revert the one change.
