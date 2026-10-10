# Proposal: Web command-reference browser

## Why

`GET /v1/admin/commands` already exposes every catalog row — all 234
Clap paths with availability (`web`, `cli_only`, `not_yet_web`,
`provider_required`, `project_capability_required`, `disabled`), a
plain-language `reason` and the exact `cli_invocation` on every non-web
row. The browser fetches this catalog once (`loadCommands`) but only
uses the `web` rows to derive per-project action cards; an operator in
the browser can never discover what the browser *cannot* do, why, or
what to run in the terminal instead. The CLI-only majority of Forge is
invisible from the dashboard. This is web UI/UX audit gap 1.

## What Changes

- A read-only **Command reference** section on the existing projects
  view (`frontend/index.html` `#view-projects`), rendered from the
  already-fetched `GET /v1/admin/commands` JSON — no new endpoint, no
  new fetch, no new dependency.
- Every catalog row is listed with its availability badge, summary,
  plain-language reason (non-web rows) and exact `forge ...` CLI string
  with a clipboard Copy button (existing clipboard + select-fallback
  pattern). Non-web rows never render an executable control.
- `web` rows link to their existing view (`/projects`, `/workbench`,
  `/management`, `/portfolio`, `/delivery`) via a route→view mapping;
  no new route, so `src/web.rs` and the navigation contract are
  untouched.
- Search (id/label/summary/CLI/reason/category) + availability filter
  (all six states) with an honest count line and empty states for
  "catalog unavailable" vs "no matches".
- New static contract test
  `tests/web_command_reference_browser_contract.rs` pinning the
  section/controls tokens, badge+reason+CLI rendering, web-row links,
  and the no-shell/no-write boundary.

## BFS Impact Map

- **Requirements/concepts:** `portal-web-ui` (standalone `frontend/`
  SPA); `forge-command-catalog/0.1.0` contract (reused read-only).
- **Modules:** `frontend/index.html` (one section + controls),
  `frontend/app.js` (render + filter + copy, reusing `catalogCommands`,
  `AVAILABILITY_LABELS`, `AVAILABILITY_BADGE`, `CATEGORY_LABELS`,
  clipboard fallback), `frontend/styles.css` (one additive block).
- **Contracts/callers:** none changed — `GET /v1/admin/commands`
  shape, `src/web.rs` allowlist, router, catalog rows, journal,
  registry all untouched. `loadCommands` gains one render call.
- **Persistence/integrations:** none. Read-only view over static
  metadata; clipboard write is the only browser-side effect.
- **Tests:** new contract test only; existing suites
  (`portal_ui_contract`, `forge_web_command_catalog_contract`,
  `forge_web_navigation_contract`) must stay green unchanged.
- **Compatibility:** old API without the catalog → honest
  "unavailable" empty state (existing `loadCommands` catch path).
- **Concerns:** quality (success/failure/empty states mapped);
  security (catalog strings rendered via `textContent` only, never
  `innerHTML`; no shell, no path/command submission, no secret
  handling — CLI strings come from the static catalog).

## Capabilities

- Operators can browse, search and filter every command by
  availability from the dashboard.
- Operators can copy the exact terminal string for anything the
  browser cannot do, with its plain-language reason beside it.
- Screen-reader and keyboard operators get the same reference
  (native controls, live count, per-copy status announcement).

## Non-goals

- No new route (`/reference` or otherwise), no `src/web.rs` change, no
  sidebar change (fragment nav is contract-forbidden).
- No browser execution of non-web rows; no new writes, endpoints, or
  shell/eval surface anywhere.
- No per-row parameter forms or preview/confirm flows — executable
  `web` rows already have cards on their own views; the reference only
  links there.
- No new frontend dependency; no styling framework change.
