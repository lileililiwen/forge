# Design: Web creation-catalog browser

## 1. Ownership and placement

The browser is a section of the **projects view**, not a new view —
the gap-1 reference pattern (`web-command-reference-browser` design
§1). `src/web.rs::asset_name` stays an exact allowlist; no route,
sidebar, `VIEW_BY_PATH`/`VIEW_IDS`/`VIEW_CRUMBS` or navigation-contract
change. Placement: `<section id="creation-catalog">` inside
`#view-projects`, after `#catalog-browser`, before `#workbench`, so
every signed-in operator lands on it and it never interferes with the
workbench, management, portfolio or delivery views.

Owner: backend `src/api/` (thin read handlers + wiring) and
`frontend/` assets. No CLI/MCP/registry/journal/Core-logic change.

## 2. Route contracts (all GET, session-gated, read-only)

Constants live in `src/api/admin/routes.rs` and are re-exported
through `command_catalog/routes.rs` so the catalog and the live
endpoints can never name different paths. One `Route::AdminCreation
{ registry, item, action }` variant carries a validated-key triple
(portfolio `{kind}`/`{action}` precedent); the `route_creation`
matcher in `src/api/admin/creation.rs` constructs only valid combos
and the handler re-validates the allowlists (defense in depth).
`router.rs` (998/1000 lines at baseline) grows by exactly one call
line: the pre-table chain folds into a `route_beside` helper in
`src/api/admin/mod.rs` that tries the history, agent/identity and
creation matchers in order, and the permission/short-circuit/
exhaustiveness entries append to existing arms. The six reachable
creation user-error codes (`unsupported-profile`,
`component-invalid`, `ui-pattern-invalid`,
`ui-pattern-unsupported-platform`, `standard-invalid`,
`procedure-invalid`) map to `400` in a `creation_error_status`
helper beside the handlers — not in the shared `err_status` table —
so `router.rs` stays ≤1000 lines; everything else keeps its mapping.

| Route constant | Method + path | Core service | Response |
|---|---|---|---|
| `ROUTE_ADMIN_CREATION_PROFILES` | `GET /v1/admin/creation/profiles` | `profile::list_profiles` | `{ creation: { registry, entries }, contract }` in stable id order |
| `ROUTE_ADMIN_CREATION_PROFILE` | `GET /v1/admin/creation/profiles/{id}` | `profile::inspect_profile` | `{ creation: { registry, entry }, contract }`; planned ids stay discoverable with `support_status: planned` |
| `ROUTE_ADMIN_CREATION_PROFILE_RESOLVE` | `GET /v1/admin/creation/profiles/resolve?id=&feature=` | `profile::resolve_profile` | `{ creation: { registry, resolution }, contract }` (`ResolvedProfile`: id/version/adapter/language/toolchain) |
| `ROUTE_ADMIN_CREATION_FEATURES` (+`_FEATURE`, `+_FEATURE_RESOLVE`) | `.../creation/features[/{id}][/resolve?profile=&feature=]` | `feature::feature_catalog|inspect_feature|resolve_plan` | entries / entry (`FeatureDescriptor`) / `FeaturePlan` (exact versions in dependency order) |
| `ROUTE_ADMIN_CREATION_COMPONENTS` (+`_COMPONENT`, `+_COMPONENT_RESOLVE`) | `.../creation/components[/{id}][/resolve?profile=&component=]` | `component::component_catalog|inspect_component|resolve_outcome` | entries / entry / `ComponentResolveOutcome` (plan + evidence summary + note; rejections stay reviewable, never hidden) |
| `ROUTE_ADMIN_CREATION_UI_PATTERNS` (+`_UI_PATTERN`, `+_UI_PATTERN_RESOLVE`) | `.../creation/ui-patterns[/{id}][/resolve?profile=&pattern=]` | `ui_pattern::ui_pattern_catalog|inspect_ui_pattern|resolve_outcome` | entries / entry / `UiPatternResolveOutcome` |
| `ROUTE_ADMIN_CREATION_STANDARDS` (+`_STANDARD`) | `.../creation/standards[/{id}]` | `standard::all_packs|inspect_pack` | entries (`PackDescriptor`) / entry for `<pack>@<version>` |
| `ROUTE_ADMIN_CREATION_PROCEDURES` (+`_PROCEDURE`) | `.../creation/procedures[/{id}]` | `procedure::procedure_catalog|inspect_procedure` | entries / entry (prerequisites, ordered steps, verification) |
| `ROUTE_ADMIN_CREATION_INTENT_VALIDATE` | `GET /v1/admin/creation/intents/validate?action=&profile=&require=&forbid=&constraint=` | intent action parse + `validate_intent` + `intent_hash` | `{ creation: { validated, intent_hash }, contract }`; journal-free (the CLI's `record_operation("planner",…)` line is deliberately not reproduced) |
| `ROUTE_ADMIN_CREATION_STANDARD_CHECK` | `GET /v1/admin/projects/{id}/standard/check` | `deploy_id_gate` + `standard::check_snapshot` | `{ creation: { project_id, report }, contract }`, path-scrubbed |
| `ROUTE_ADMIN_CREATION_STANDARD_DIFF` | `GET /v1/admin/projects/{id}/standard/diff?against=<pack>@<version>` | `deploy_id_gate` + `standard::diff_snapshot` | `{ creation: { project_id, against, report }, contract }`, path-scrubbed |
| `ROUTE_ADMIN_CREATION_INTENT_PLANS` | `GET /v1/admin/projects/{id}/intent/plans` | `deploy_id_gate` + read `.forge/planner/` dirs | `{ creation: { project_id, plans: [{ plan_id }] }, contract }` — plan ids only, never the CLI's absolute receipt path |

Query parsing reuses the `percent_decode` helper; repeatable keys
collect in order (`feature`, `component`, `pattern`, `require`,
`forbid`, `constraint`); a missing required subject (`id`,
`profile`, `action`, `against`) is a static typed `400` that never
echoes input. `constraint` uses the CLI's `key=value` rule
(malformed → `intent-invalid`, same message shape as the CLI).

Guards (in this order): exact-origin check → `guarded()` admin
session (401 without) → hostile-segment gate on `{id}` (blank, `/`,
`\`, `..`, `%` → static `400`, never echoed) → Core call →
`ApiResponse::json(200)`. Unknown entries → Core's typed refusal via
`from_error` (codes preserved, so the boundary stays honest);
project-bound failures scrub the server-resolved directory.

Matcher ordering: the literal `resolve` arm precedes the `{id}` arm
per registry so `resolve` never reads as an entry id; the `creation`
literals never collide with the `projects`/`portfolio`/`delivery`
arms, and the `projects/{id}/standard/*` + `projects/{id}/intent/
plans` literals never collide with the existing `feature`/`spec`/
`deploy`/`release`/`publish`/`delivery`/`agents`/`identity`/
`releases`/`deploys` arms — all differing in method or literal.
`OPTIONS ["v1","admin","creation",..]` returns `AdminOptions` from
the same matcher; the six-segment project arms reuse the existing
six-or-more project OPTIONS tail. `required_permission` returns
`None` (same posture as every other admin read: any valid admin
session). `err_status` maps the six reachable user-error codes to
`400` (all other mappings untouched).

## 3. Catalog-row conversion (count stays 234)

Twenty `leaf(NotYetWeb)` rows become `web_at(Read, <route>,
caps)` with capabilities kept verbatim (`none` for pure catalogs,
`caps_local` where the CLI row already declares it):

- `profile.list|inspect|resolve`, `feature.list|inspect|resolve`,
  `component.list|inspect|resolve`,
  `ui-pattern.list|inspect|resolve` (12, `none` caps).
- `standard.list|inspect` (`none`), `standard.check|diff`
  (`caps_local`), `procedure.list|inspect` (`none`),
  `intent.validate` (`none`), `intent.list` (`caps_local`).
- `problems()` stays empty: every new `web` row names a route in
  `IMPLEMENTED_WEB_ROUTES` (+20 entries).

Untouched: `profile.preflight` (`cli_only`, native), `procedure.
validate` (`cli_only`, file-stdin), `component.qualify`,
`ui-pattern.install`, `standard.upgrade` (`NotYetWeb` writes), all
other writes, and `intent.resolve|apply` (already web-exec).

## 4. Frontend rendering

One `#creation-catalog` section: a registry picker (six listable
registries), a search box (client-side substring filter over id +
summary text, same pattern as `cmdrefMatches`), per-registry List /
inspect (id input) / resolve (profile + repeatable ids) controls,
a standards project panel (project select + check button + diff
`against` input), an intent panel (validate form + per-project plans
button), and a CLI-only remainder rendered from the already-loaded
`catalogCommands` (creation-category rows whose availability is not
`web`: badge + reason + CLI string with Copy, reusing `cmdrefRow`
styles — never an executable control). Results render through `el()`
/ `textContent` only into `role="status"` live regions with an
error-summary + focus; project-bound controls with no chosen project
show the honest "choose a project" note; a failed GET (e.g. an old
API without the routes) renders "Unavailable — <typed reason>".
Every fetch is an explicit button click (no auto-load); ids travel
via `encodeURIComponent`; no `eval`/`Function`/`innerHTML`/shell.

## 5. Failure and compatibility

- Unknown registry key → matcher returns `None` → `route-not-found`
  404 (a key is not a path, never opened).
- Unknown entry → Core typed refusal, code + status preserved.
- Planned profile resolve → `unsupported-profile` 400 (same as CLI).
- Resolve with rejections (component/ui-pattern) → 200 with the
  outcome including `rejections` (CLI parity: partial plans stay
  reviewable; only malformed input is a refusal).
- Malformed `constraint` / unknown action → `intent-invalid` 400.
- Missing `.forge/planner/` → `{ plans: [] }` 200 (an empty catalog
  is not an error — source-bundle precedent).
- Unreadable plans dir → 503 `admin-api-unavailable` (honest, never
  a fake empty list).
- Registry bytes identical before/after every read (contract test
  asserts).
