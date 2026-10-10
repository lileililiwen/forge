# Proposal: Web creation-catalog browser

## Why

The creation catalogs — `profile list|inspect|resolve`,
`feature list|inspect|resolve`, `component list|inspect|resolve`,
`ui-pattern list|inspect|resolve`, `standard list|inspect|check|diff`,
`procedure list|inspect`, `intent validate|list` — are CLI-only: all
twenty command-catalog rows sit at `not_yet_web`. The dashboard's
command reference shows them as "Not in web yet" with a terminal
fallback, and the projects view has no creation-catalog section at
all. An operator auditing "which profiles/features/components/patterns/
standards/procedures exist, are they compatible, and what would a
resolve produce" must drop to the terminal. This is web UI/UX audit
gap 6.

The resolves are pure deterministic Core functions of
(catalog, request, profile) — no filesystem, no toolchain probe, no
provider, no shell — so they are safe to expose as read-only GETs.
The project-bound reads (`standard check|diff`, `intent list`) resolve
the project directory server-side from a validated id, exactly like
the existing history routes. Everything else that writes or touches
the native/file boundary stays CLI-only by design.

## What Changes

- Twenty `not_yet_web` rows become `web` with typed read-only
  session-gated admin GET routes (no writes, no provider, no adapter,
  no shell, no journal row, no browser-supplied path):
  - Pure catalogs (no project needed), under
    `GET /v1/admin/creation/{registry}` (list),
    `GET /v1/admin/creation/{registry}/{id}` (inspect),
    `GET /v1/admin/creation/{registry}/resolve` (resolve, query-driven):
    `profiles` (`profile::list_profiles|inspect_profile|
    resolve_profile`), `features` (`feature::feature_catalog|
    inspect_feature|resolve_plan`), `components`
    (`component::component_catalog|inspect_component|
    resolve_outcome`), `ui-patterns`
    (`ui_pattern::ui_pattern_catalog|inspect_ui_pattern|
    resolve_outcome`), `standards` (list+inspect only via
    `standard::all_packs|inspect_pack`), `procedures` (list+inspect
    only via `procedure::procedure_catalog|inspect_procedure`).
  - Intent validation (pure, journal-free):
    `GET /v1/admin/creation/intents/validate` with
    `?action=&profile=` + repeatable `require`/`forbid`/`constraint`
    (same shapes as the CLI flags), reusing `parse_intent_action`
    semantics + `validate_intent` + `intent_hash`.
  - Project-bound reads (directory resolved server-side via the
    existing `deploy_id_gate`, responses scrubbed of the absolute
    path): `GET /v1/admin/projects/{id}/standard/check` and
    `.../standard/diff?against=<pack>@<version>` (reusing
    `standard::check_snapshot|diff_snapshot`), and
    `GET /v1/admin/projects/{id}/intent/plans` (plan ids under the
    project's `.forge/planner/`, path-free by design — the CLI's
    absolute receipt path is never echoed).
- One new `Route::AdminCreation { registry, item, action }` variant
  (validated-key triple, portfolio `{kind}`/`{action}` precedent) plus
  a `route_creation` matcher kept beside the handlers in the new
  `src/api/admin/creation.rs` — never in the `router.rs` table, which
  sits at 998/1000 lines. `router.rs` grows by exactly one chain line;
  permission/short-circuit/exhaustiveness entries append to existing
  lines. `err_status` gains the five reachable user-error codes
  (`unsupported-profile`, `component-invalid`, `ui-pattern-invalid`,
  `ui-pattern-unsupported-platform`, `standard-invalid`,
  `procedure-invalid`) as `400`s; everything else keeps its mapping.
- A read-only **Creation catalog** section on the existing projects
  view (`frontend/index.html` `#view-projects`, after
  `#catalog-browser`): registry picker + search, per-registry
  list/inspect/resolve-or-check/diff rendering, project-bound controls
  gated on a chosen project, CLI-only leftovers rendered from the
  loaded command catalog as availability badges with reasons, and
  honest unavailable-with-reason empty states. No new frontend
  dependency; existing styles/a11y patterns reused.
- New contract tests:
  `tests/web_creation_catalog_browser_contract.rs` (static frontend
  tokens + live admin-route round-trips) plus pin updates in the
  command-catalog web vec and the command-catalog contract allowlists.

## BFS Impact Map

- **Requirements/concepts:** profile/feature/component/ui-pattern/
  standard/procedure/intent Core contracts, `portal-web-ui`
  (standalone `frontend/` SPA).
- **Modules:** `src/api/model.rs` (1 `Route` variant),
  `src/api/router.rs` (1 chain line + same-line permission/
  short-circuit/exhaustiveness/`err_status` appends),
  `src/api/admin/routes.rs` (20 route constants),
  `src/api/admin/creation.rs` (new: matcher + guarded read handlers),
  `src/api/admin/deploy.rs` (1 dispatch arm),
  `src/api/command_catalog/routes.rs` (`IMPLEMENTED_WEB_ROUTES` +20
  route refs), `src/api/command_catalog/rows_project.rs` (20 rows
  `NotYetWeb` → `web_at`), `frontend/index.html` (one section),
  `frontend/app.js` (fetch + render + search), `frontend/styles.css`
  (one additive block).
- **Contracts/callers:** `GET /v1/admin/commands` envelope unchanged
  (20 rows change state, count stays 234 — conversion, not addition);
  CLI/MCP surfaces untouched; `loadCommands`, workbench, portfolio,
  delivery untouched.
- **Persistence/integrations:** none. Pure-catalog reads touch no
  filesystem at all; project-bound reads open the registry read-only
  and read the project tree; no registry byte, table row or journal
  row is written (the CLI's `intent validate` journal line is
  deliberately not reproduced); no provider, adapter, native
  toolchain, or shell.
- **Tests:** new `web_creation_catalog_browser_contract`; existing
  `forge_web_command_catalog_contract` (allowlist pins grow by 20),
  `portal_ui_contract`, `forge_web_navigation_contract`,
  `web_command_reference_browser_contract`, plus the CLI parity suites
  for profile/feature/component/ui-pattern/standard/procedure/intent
  must stay green.
- **Compatibility:** old API without the routes → honest
  "unavailable" empty states; unknown id → typed Core refusal
  (`unknown-profile`, `unknown-feature`, `component-invalid`,
  `ui-pattern-invalid`, `standard-invalid`, `procedure-invalid`,
  `intent-invalid`, `unknown-project`); hostile segments → static
  `400`s that never echo; 401 without session.
- **Concerns:** quality (success/failure/empty states mapped);
  security (session-gated, origin-checked, `textContent`-only
  rendering, no shell/path/command submission, project paths
  scrubbed, plan receipts path-free).

## Capabilities

- Operators can browse all six creation catalogs with
  list/inspect/resolve from the dashboard without the terminal.
- Operators can validate a structured intent and read a project's
  standard check/diff plus persisted plan receipts from the browser.
- Operators still see honest CLI-only badges (with reasons and exact
  terminal commands) for every write and native/file-bound verb.

## Non-goals

- Any install/qualify/upgrade/apply write: `component qualify`,
  `ui-pattern install`, `standard upgrade`, `feature add|remove|
  upgrade`, `intent resolve|apply` (already web) — untouched, stay
  `NotYetWeb`/`cli_only`/web-exec respectively.
- `profile preflight` and `procedure validate` (native toolchain /
  file-stdin boundary) — stay `cli_only`, untouched.
- No new filtering semantics: resolve query keys mirror the CLI flags
  exactly; list responses are the full stable-order catalogs.
- No new route beyond the twenty reads; no new view/route in
  `src/web.rs`; no sidebar change; no new frontend dependency.
