# Design: Web project-catalog browser

## 1. Ownership and placement

The browser is a section of the **projects view**, not a new view —
the gap-1 reference pattern (`web-command-reference-browser` design
§1). `src/web.rs::asset_name` stays an exact allowlist (`/projects`,
`/workbench`, `/management`, `/portfolio`, `/delivery`); no route,
sidebar, `VIEW_BY_PATH`/`VIEW_IDS`/`VIEW_CRUMBS` or navigation-contract
change. Placement: `<section id="catalog-browser">` inside
`#view-projects`, after `#command-reference`, before `#workbench`, so
every signed-in operator lands on it and it never interferes with the
workbench, management, portfolio or delivery views.

Owner: backend `src/api/` (6 thin read handlers + wiring) and
`frontend/` assets. No CLI/MCP/registry/journal change.

## 2. Route contracts (all GET, session-gated, read-only)

Constants live in `src/api/admin/routes.rs` and are re-exported so
`command_catalog/routes.rs` names the identical strings:

| Route constant | Method + path | Core service | Response |
|---|---|---|---|
| `ROUTE_ADMIN_CATALOG` | `GET /v1/admin/projects/catalog` | `collect` + `CatalogQuery::from_pairs` + `apply` | `{ catalog: CatalogPage, contract }` — same bytes as `GET /v1/projects/catalog` |
| `ROUTE_ADMIN_CATALOG_TAGS` | `GET /v1/admin/projects/catalog/tags` | `collect` + `filter` + `tag_counts` | `{ catalog: { contract, tags: [...] }, contract }` — same shape as CLI `project tags --format json` |
| `ROUTE_ADMIN_CATALOG_LANGUAGES` | `GET /v1/admin/projects/catalog/languages` | `collect` + `filter` + `language_counts` | `{ catalog: { contract, languages: [...] }, contract }` |
| `ROUTE_ADMIN_CATALOG_GAPS` | `GET /v1/admin/projects/catalog/gaps` | `collect` + `inspect_records?` + `gaps::build_report` + `GapFilters` | `{ gaps: { contract, project_id, summary, findings }, contract }` — same shape as CLI `project gaps --format json` |
| `ROUTE_ADMIN_CATALOG_INSPECT` | `GET /v1/admin/projects/{id}/catalog` | `collect` + `inspect_records` | `{ catalog: { contract, project_id, records }, contract }` — same bytes as `GET /v1/projects/{id}/catalog` |
| `ROUTE_ADMIN_FLEET_INSPECT` | `GET /v1/admin/fleet/{entry}` | `fleet::observe` + `inspect_entry` | `{ contract, entry, source, freshness, observed_at }` — same shape as CLI `fleet inspect --format json` |

Query params reuse `parse_catalog_query_params` /
`build_catalog_selection` / `catalog_filter_pairs_from` (limit, cursor,
`tag|language|profile|lifecycle|repository|ci|compose|evidence`,
`filter=key=value`, `source`, `max-age`, `workspace-registry`,
`inventory`, `git-repository`, `github-repository`). Gaps adds
`project`, repeatable `category` (`description|tags|ci|compose|
manifest|docs|repository`), `status`
(`pass|warn|fail|unavailable|not_applicable`), `remediation-class`
(`automatic|semantic|manual`) — parsed exactly as
`src/cli/gaps.rs` (`GapCategory::parse`, `GapStatus::parse`,
`RemediationClass::parse`); unknown values are typed
`catalog-invalid` 400s, never silent. Fleet inspect takes only
`max-age` (default `DEFAULT_MAX_AGE_SECONDS`); the workspace registry
resolves server-side via `resolve_registry_path(None)` — the browser
never sends a path.

Guards (in this order): exact-origin check → `guarded()` admin session
(401 without) → `validate_project_id` on `{id}`/`{entry}` (400 on
hostile input, never echoed) → Core call → `ApiResponse::json(200)`.
Unknown project/entry → Core's typed `unknown-project` (mapped by
`from_error`, no invented record). Every handler opens the registry
read-only and writes nothing (no journal row — reads never call
`run_with_operation`).

Router: literal `catalog` arms precede the generic
`["v1","admin","projects", id]` arm so `catalog` never reads as an id;
`{id}/catalog` is a 5-segment literal arm alongside
`status`/`plan`/`maintain`; `fleet/{entry}` is a disjoint 4-segment arm
(`GET /v1/admin/status` is fleet *health*, untouched). OPTIONS arms for
the new depths mirror the existing catalog-adjacent preflights.
`required_permission` returns `None` for all six (same posture as every
other admin read: any valid admin session, no extra permission —
`AdminOptions`-style opted-out arm, consistent with `AdminProjects`).
`IMPLEMENTED_WEB_ROUTES` gains all six; `rows_fleet.rs` converts the
six `leaf(NotYetWeb)` rows to `web_at(Read, <route>, caps_registry)`.

## 3. Catalog-row conversion (count stays 234)

- `fleet.inspect` → `web_at(Read, WEB_ROUTE_ADMIN_FLEET_INSPECT)`.
- `project.list` → `web_at(Read, WEB_ROUTE_ADMIN_CATALOG)`.
- `project.inspect` → `web_at(Read, WEB_ROUTE_ADMIN_CATALOG_INSPECT)`.
- `project.tags` → `web_at(Read, WEB_ROUTE_ADMIN_CATALOG_TAGS)`.
- `project.languages` → `web_at(Read, WEB_ROUTE_ADMIN_CATALOG_LANGUAGES)`.
- `project.gaps` → `web_at(Read, WEB_ROUTE_ADMIN_CATALOG_GAPS)`.

`caps_registry` (`registry_read`) is kept verbatim. `problems()` stays
empty: every `web` row names a route in `IMPLEMENTED_WEB_ROUTES`.

## 4. Frontend rendering

`initCatalogBrowser()` wired once at dashboard boot; six fetchers using
the existing `request()` helper (cookie session, 401 → existing login
redirect):

- `#catalog-project` (select of `window.__forgeProjects` identities) +
  `#catalog-inspect` button → `GET /v1/admin/projects/{id}/catalog` →
  `#catalog-inspect-result` (`role="status"`): one block per record
  (`project_id`, `source`, `source_kind`, `source_revision`,
  `observed_at`, `freshness`, `profile`, `lifecycle`, `repository`,
  `languages`, `tags`, `ci`, `compose`, `evidence`).
- `#catalog-tags-refresh` → tags route → `#catalog-tags` table
  (value/count); `#catalog-languages-refresh` → languages route →
  `#catalog-languages` table. Both honor the six fleet predicate inputs
  as query params (same encoding as `fleetFilterParams`).
- `#catalog-gaps-refresh` (+ `#catalog-gaps-project` optional text,
  `#catalog-gaps-category/status/class` optional selects) → gaps route →
  `#catalog-gaps` list: `id`, `project_id`, `category`, `status` badge,
  `remediation_class`, evidence (`source`, `source_revision`,
  `observed_at`), plus summary line (`total`, `returned`, `clean`).
- `#catalog-fleet-entry` (text) + `#catalog-fleet-inspect` button →
  `GET /v1/admin/fleet/{entry}` → `#catalog-fleet-result`
  (`role="status"`): entry fields + `source`/`freshness`/`observed_at`.

Empty states: route 401/unreachable → "Catalog browser unavailable…";
valid 200 with zero rows → "(none)" honest empties; unknown id →
typed error text, never a silent table. All strings via `textContent` /
existing `el()` helper — never `innerHTML`, `eval`, `Function`,
`child_process`, shell. Single-fragment appends per list.

## 5. Failure boundaries

| Failure | Rendering / status |
|---|---|
| No session (401) | existing login redirect; section shows unavailable state |
| Unknown project/entry | typed error text (`unknown-project` / fleet refusal), nothing rendered as data |
| Invalid filter/category/status/class | typed `catalog-invalid` text, prior results kept (never replaced by unfiltered data) |
| Empty catalog / no tags / no gaps | explicit "(none)" / "No gaps — clean" (never healthy-by-silence) |
| Unavailable source | named `unavailable` row with reason (from `sources` array), other sources still shown |

## 6. Accessibility and style contracts

- Native `<select>`, `<input>`, `<button>`, `<table>` — keyboard
  operable with visible focus (existing tokens).
- Result containers + count lines are `role="status"` live regions via
  existing `setResultRole`; error-summary focus pattern reused for the
  inspect forms (existing `.error-summary` + focus).
- 12px floor: browser text uses `--text-md` or larger; only badges keep
  the compact `.badge` chip size (gap-1 pattern).
- No motion added; no layout shift (controls reserve their row).
- Reuses `makeBadge`, `AVAILABILITY`/`HEALTH` badge tables where a
  verdict chip is needed, `textCell`/`detailRow` row helpers.

## 7. Explicit non-changes

`src/web.rs`, sidebar, `VIEW_BY_PATH`, CLI/MCP/catalog-Core/gaps-Core/
fleet-Core logic, registry schema, journal, providers/adapters, shell,
`GET /v1/projects/catalog` family, `inventory show`, `fleet online`,
`project github *` — all untouched. The change adds exactly: 6 route
constants + 6 `Route` variants + router arms + 1 handler module (or
equivalent guarded arms) + catalog-route refs + 6 row conversions +
one HTML section + one CSS block + one app.js function group + one
test file.
