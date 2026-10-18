# Design: make project quality evidence the primary portal view

## Implementation boundary

Modify Forge's `src/portal/model.rs` and `src/portal/sections.rs` to render the
stable legacy `settings` section as a Gate quality summary. In
`frontend/index.html` and `frontend/app.js`, make the existing workbench Gate
view a dedicated Quality card, correctly unwrap its API response, provide
state-specific next-step text, and collapse the Assurance tools under
Advanced diagnostics. Update `frontend/styles.css`, portal/workbench tests,
browser harness, `docs/portal-spa.md`, the three named canonical specs, the
roadmap and HANDOFF. No new route, persistent data, runtime, project template,
or sibling source is in scope.

## Language and runtime

Rust 2021 and the existing Forge Core/CLI; static HTML, CSS and vanilla
JavaScript served by `forge web`; the pinned Playwright 1.63.0 browser harness
in `tests/browser`. Run focused Rust contracts, `node --check
frontend/app.js`, the signed-in Chromium quality flow, strict OpenSpec checks,
and the required bounded Forge Gate. No new dependency or browser framework.

## Ownership and shared code

Forge owns the portal, the public `PortalSection` mapping and workbench. Gate
evidence remains owned by Forge's existing Gate module and API. The UI
consumes `GET /v1/admin/projects/{id}/gate/status` and the existing explicit
`POST /v1/admin/projects/{id}/gate/run`; it neither reads the Governance
module directly nor invents a second evidence projection. The server-side
`settings` identifier remains stable for compatibility; only its human label,
summary source, help text and SPA route change. No shared sibling UI package
exists or should be extracted for this Forge-only operator surface.

## User experience and interface

- **Entry:** Projects → open a managed project → Workbench → Quality checks.
- **Hierarchy:** one aggregate and freshness heading; a short next step; the
  existing revision/time/summary facts; an ordered check list; the existing
  Re-run gate button.
- **Initial/loading:** announce `Loading quality checks…`; with no project,
  say to open a project; never use a blank card.
- **Absent:** label `Not run`, state that unverified is not a pass, and direct
  the operator to the existing Re-run gate button.
- **Fresh/pass:** show `Passed` and `Fresh`; explain it applies to the current
  revision and should be rerun after project changes.
- **Stale:** preserve the recorded aggregate but label `Stale`; explain that
  it belongs to another revision and rerun is needed for current evidence.
- **Blocked/failed/unknown:** show the actual result and checks; direct the
  operator to review the listed checks and rerun after fixing the cause.
- **Unavailable/error:** show the safe existing error/recovery notice; never
  display a pass or discard a prior persisted result.
- **Advanced:** Projects → Assurance → expand the native `details` disclosure
  named `Advanced diagnostics and records`. Governance lists/status/inspect
  are still explicit reads and render only inside this disclosure.
- **Responsive/accessibility:** preserve native buttons and `details`, keyboard
  focus, visible labels, current `role=status`/`aria-live`, the 12px text floor,
  and the 390px no-overflow layout. Add no animation.

## Behavioral model

`renderGateStatus` accepts either the status response `{summary, envelope}` or
the run response's equivalent, normalizes to the inner envelope once, and
renders the same Gate object for both paths. State priority is:

| Input | Display | Next step |
|---|---|---|
| no Gate record / freshness `absent` | Not run / Unverified | Run Gate to record checks |
| `fresh` + `passed` | Passed / Fresh | No action unless project files changed |
| `stale` | recorded aggregate / Stale | Rerun to refresh current evidence |
| `blocked` or `failed` | actual aggregate and check rows | Review failures, fix, rerun |
| `unknown` or unresolved check | Unknown | Review check rows and rerun |
| API/runtime error | Unavailable with safe reason | Retry the existing Gate action or status read |

The portal `settings` projection reads only persisted Gate evidence and the
current revision. It maps `fresh+passed` to `ok`, fresh `blocked` to `warn`,
fresh `failed` to `fail`, stale/unknown/absent to `unknown`, and read/parse
errors to `unavailable`. It includes only aggregate, freshness and observation
time; check details are in the workbench. No provider identity or filesystem
path appears in the summary. The existing portal contract version and section
id do not change.

## Contract and compatibility

No API request or response fields change. The `PortalSection::Settings` enum
variant and serialized `settings` id remain stable. Its displayed label
becomes `Quality`, its SPA pointer becomes `/workbench`, and its controls name
`forge gate status <project>` and `forge gate <project>`. Existing serialized
entry fields are reused; `source` is `gate`, `freshness` is an additive
attribute and evidence contains bounded summary facts. Governance details
remain available only in the expanded Assurance disclosure.

## Failure and boundary policy

Missing or stale Gate evidence is never considered healthy. A malformed or
unreadable evidence record projects `unavailable` without serializing its raw
error, which may contain a project path. A per-project state failure does not
prevent other portal entries from rendering. The UI uses text nodes only and
continues to use the admin session and server-resolved project id. Page load
does not trigger a Gate run, provider observation, write, or journal event.

## Verification oracle

- Rust tests cover all portal aggregate/freshness mappings and absence/error
  states, including no provider identifier or project path in output.
- Static frontend contracts cover the Quality card, envelope normalization,
  state-specific next steps, single existing Gate action and advanced
  disclosure.
- A signed-in Chromium flow sees the persisted per-check result and freshness,
  verifies the Quality section at desktop and 390px, and expands/collapses
  Advanced diagnostics by keyboard.
- Focused Rust/API tests, JavaScript syntax, OpenSpec strict validation,
  `git diff --check`, Gate dry-run and full Gate evidence are recorded in
  HANDOFF.

## Decision ledger

- Reuse the existing Gate routes and persisted evidence; do not create a
  project-local governance UI or a second quality API.
- Keep the legacy section id `settings` to avoid breaking scripts and JSON
  consumers while changing its operator-facing meaning to Quality.
- Keep Governance reads available for advanced operators, collapsed by
  default, and never run them as a side effect of rendering the page.
