# Proposal: Make handler-backed project authoring commands executable from the browser

## Why

requirement.md §36 is authoritative: *"The portal should consume the same Forge
Core APIs as CLI and MCP."* The delivered command center already lets a signed-in
operator inspect, run `doctor`, and preview/confirm `upgrade` through the
workbench, and can approve/publish/reconcile through delivery controls. But two
project authoring commands that already have safe, typed, in-process Core
handlers — `forge feature add` (`src/api/mod.rs::handle_add_feature`) and
`forge spec generate` (`handle_generate_spec`) — are reachable only through the
bearer-token `/v1/projects/{id}/…` routes. The cookie-session global admin
surface the browser uses (`/v1/admin/…`) never exposes them, and the catalog
marks those rows `not_yet_web` with a "run it in a terminal" next step. A
signed-in operator therefore sees the commands but cannot perform them, which is
the reported requirement mismatch.

## What Changes

- Add typed, session-gated admin execution routes for the two handler-backed
  authoring commands: `POST /v1/admin/projects/{id}/feature` (feature add) and
  `POST /v1/admin/projects/{id}/spec` (spec generate). Each delegates to the SAME
  in-process Core handler the CLI and `/v1` bearer routes already use.
- Require a `confirm` flag plus a `plan_digest` binding on both, enforced in the
  admin layer (the bearer handlers do not gate this), mirroring the existing
  workbench/delivery confirm+digest guard. The browser first requests a bounded
  preview so it can obtain the digest, then submits `confirm: true` with that
  exact digest.
- Reclassify those two catalog rows from `not_yet_web` to `web`, each pointing at
  its real admin route, and add the routes to `IMPLEMENTED_WEB_ROUTES` so the
  catalog's evidence-based web check accepts them.
- Add a browser preview-and-confirm workflow in `frontend/` for these actions:
  choose the feature id (or finding ids + reason), request a bounded preview,
  review the returned digest, then confirm; render the typed result with no
  free-form shell entry.
- Preserve every existing boundary: `doctor`/`inspect`/`upgrade` stay as the
  workbench already delivers them; transport/build/PTY and interactive `agent`
  commands keep their honest CLI-only disposition; publish/deploy stay behind the
  existing delivery controls; no generic "run this CLI text" endpoint is added.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-web-command-execution` | A signed-in operator can execute `feature add` and `spec generate` from the browser through typed, confirm+digest-bound admin routes, and the catalog reports those two rows as `web`. | Forge / Rust 2021, minimum Rust 1.87; standalone `frontend/`. | Existing Core `handle_add_feature` / `handle_generate_spec`, the admin session gate + workbench digest pattern, the catalog route constants, and `frontend/app.js`. | Archived `forge-web-project-workbench` and `forge-web-command-catalog`; no active dependencies. | A route contract test signs in, requests each preview (obtains a digest), confirms with the correct digest and checks it delegates to the Core handler, refuses an unconfirmed or wrong-digest mutation with no write, and checks the two catalog rows report `web` with a route in `IMPLEMENTED_WEB_ROUTES`; plus a browser check that both run without a shell box. |

The preview, the confirmed mutation, the catalog reclassification and the browser
workflow all describe the one capability (browser execution of these two
handler-backed commands) and cannot ship independently without a truthful catalog
or an unreachable button, so this is the smallest independently verifiable
outcome.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge admin surface | `src/api/admin.rs::{handle,guarded,delivery_write}`, `workbench::apply`, `plan_digest` | Session gate + confirm/digest mutation pattern already proven for workbench/delivery. | Those guards cover only workbench/delivery; the two authoring actions are not routed through admin. | Forge owns its Rust admin/`/v1` API and cookie session. | **keep local**; reuse the existing guard shape. |
| Forge Core handlers | `src/api/mod.rs::handle_add_feature`, `handle_generate_spec` | Already typed, in-process, structured-field only, auth-neutral (dispatch-layer auth). | They write without a confirm/digest; admin must add its own gate + digest. | Forge owns Core. | **keep local**; delegate, do not fork handler logic. |
| Command catalog | `src/api/command_catalog.rs::{IMPLEMENTED_WEB_ROUTES, leaf, Availability}`, `workbench::ROUTE_*` | Route constants sourced from handler modules so they cannot diverge. | The two rows are `not_yet_web`. | Forge owns the catalog. | **keep local**; reclassify only rows with a real implemented route. |

## BFS Impact Map

- **Actors and flow:** operator signs in → opens a project → chooses *Add feature*
  or *Generate spec* → submits the structured fields to a preview → reviews the
  digest → confirms → the admin route calls the Core handler → typed result renders.
- **Contracts and callers:** two new `POST /v1/admin/projects/{id}/…` routes and
  their `OPTIONS` `AdminOptions` pairs; catalog `route`/`availability` for the two
  rows; `frontend/app.js` action workflow. Existing `/v1/projects/{id}/…` bearer
  routes and CLI behavior are unchanged.
- **Modules:** `src/api/mod.rs` (Route enum, routing table, session-gate allowlist,
  mutation/is_mutating sets), `src/api/admin.rs` (guarded branches + confirm/digest),
  shared route constants, `src/api/command_catalog.rs`, `frontend/app.js`,
  `frontend/index.html`/`styles.css`, the catalog and workbench-style contract tests.
- **Persistence:** none new; Core handlers use the existing registry and project
  directories unchanged.
- **Failures and boundaries:** no session → the same 401 the other admin routes
  return; a mutation without confirm or with a mismatched digest is refused and
  performs no write; unknown project / empty findings / missing feature id return
  the Core handler's existing typed safe errors; no request field is ever turned
  into argv or a shell command.
- **Compatibility/security:** no generic text-execution endpoint; PTY/transport/
  build/`agent` commands stay CLI-only; publish/deploy stay in delivery controls;
  passwords and absolute filesystem paths stay out of responses; CORS stays
  exact-origin.
- **Tests:** per-action delegation + session-gate refusal + confirm/digest refusal
  (both no-confirm and wrong-digest paths), catalog completeness now expecting the
  two `web` rows, browser run workflow, and regression on the untouched bearer routes.
- **Unaffected:** the CLI itself, MCP, the delivery/portfolio/workbench surfaces,
  identity login, generated projects, and sibling repositories.

## Capabilities

### New Capabilities

- `forge-web-command-execution`: a session-gated, typed admin execution surface for
  the handler-backed project authoring commands, with confirm+digest-bound
  mutations and a browser preview-and-confirm workflow.

### Modified Capabilities

- `forge-web-command-catalog`: `feature add` and `spec generate`, now backed by
  implemented admin routes, report `web` with those routes instead of a CLI-only
  next step.

## Non-goals

- No endpoint that executes arbitrary CLI text, argv, or shell.
- No browser execution of transport/build/server-start, PTY or interactive
  `agent` commands; those keep an honest CLI-only disposition.
- No new Core behavior; the admin surface only exposes handlers that already exist.
- No change to publish/deploy authorization, which stays in the delivery controls.
- No change to `doctor`/`inspect`/`upgrade`, already delivered by the workbench.
