# Proposal: Lifecycle-tracked project view

## Why

The standalone web UI today has two surfaces for acting on a
project: a separate "Command catalog (reference)" page that
lists every CLI command with filters, and a per-project
"workbench" page that exposes some of the catalog as buttons
under "Project actions". Operators asked to track a project
"from new to publish or to death" had to:

1. Open the catalog, find the right command by name or filter.
2. Switch back to the project, find the right action button.
3. Repeat for every state transition.

The catalog page is a reference surface, not an operator
surface. It is useful for someone writing tooling against
the CLI; it is not useful for an operator managing one
project at a time. The number of CLI commands a project
exposes is not the operator's concern; what is the operator's
concern is what the project needs *right now, in this state*.

The new project view makes lifecycle first-class. A single
panel per project shows identity, current lifecycle state,
contextual actions (as buttons), and recent journal evidence.
The actions change as the project moves through its
lifecycle, so an operator with a project in `L0`
(`draft` / `unregistered`) sees "Create", "Import",
"Register" as buttons; in `L2` (`working`) sees "Plan
upgrade", "Apply upgrade", "Plan publish", "Apply publish",
"Mark deprecated"; in `L4` (`archived` / dead) sees
"Restore" or nothing. The number of commands a project
exposes is a derived property of its state, not a primary
input to the operator.

## What Changes

- The standalone "Command catalog (reference)" page in the web
  UI is removed: the `<section id="commands">` block in
  `frontend/index.html`, the "Commands" nav link, the
  `renderCommands` function in `frontend/app.js`, the
  `command-search` / `category-filter` /
  `availability-filter` / `command-rows` /
  `command-no-results` / `commands-table` element lookups,
  and the `.command-table` / `.command-id` / `.command-path` /
  `.command-guidance` styles in `frontend/styles.css`.
- The web UI's per-project workbench is reordered: the
  "Manage this project" card (the renamed "Project actions"
  card) is the first action surface rendered, above the
  upgrade workflow and the read-only cards (manifest,
  health, status, delivery, evidence). The "Workflows" text
  list is removed because the actions card already exposes
  the same surface as buttons.
- The web UI gains a "Lifecycle" card at the top of the
  workbench. It shows the project's current maturity level
  (`L0`…`L4`), target maturity, profile, path, and last
  update, plus a one-line description of what that maturity
  level means in the operator's view. The card sits above
  the actions card so the operator sees lifecycle state
  before buttons.
- The actions card groups one button per available action
  by lifecycle stage: Adopt, Day-to-day, Release, Retire.
  The grouping is derived from each command's catalog id
  (e.g. `feature.*`, `upgrade.*`, `release.*`, `publish.*`)
  so a future command lands in the right group without
  bespoke wiring. Each button leads to the existing
  preview → confirm flow; no new confirmation machinery.
- The catalog JSON endpoint (`/v1/admin/commands`) stays.
  The workbench already fetches it once on page load; the
  fetch is unchanged. The `forge-web-command-catalog` spec
  remains, but the spec's name now refers to the API
  surface, not a browser page.
- No CLI change. No JSON API change. No registry/journal
  schema change. The same routes the workbench already
  calls are the routes the new actions card calls.

## Why

The standalone web UI today has two surfaces for acting on a
project: a separate "Command catalog (reference)" page that
lists every CLI command with filters, and a per-project
"workbench" page that exposes some of the catalog as buttons
under "Project actions". Operators asked to track a project
"from new to publish or to death" had to:

1. Open the catalog, find the right command by name or filter.
2. Switch back to the project, find the right action button.
3. Repeat for every state transition.

The catalog page is a reference surface, not an operator
surface. It is useful for someone writing tooling against
the CLI; it is not useful for an operator managing one
project at a time. The number of CLI commands a project
exposes is not the operator's concern; what is the operator's
concern is what the project needs *right now, in this state*.

The new project view makes lifecycle first-class. A single
panel per project shows identity, current lifecycle state,
contextual actions (as buttons), and recent journal evidence.
The actions change as the project moves through its
lifecycle, so an operator with a project in `L0`
(`draft` / `unregistered`) sees the Adopt group of actions;
in `L2` (`working`) sees the Day-to-day and Release groups;
in `L4` (`archived` / dead) sees the Retire group. The
number of commands a project exposes is a derived property
of its state, not a primary input to the operator.

## Package Boundary and Split Assessment

| Package | Outcome | Boundary | Oracle |
|---|---|---|---|
| `lifecycle-tracked-project-view` (this) | Per-project workbench has identity + lifecycle state + contextual action buttons first; the catalog-as-reference page is gone | `frontend/index.html` (project workbench + nav + commands section), `frontend/app.js` (catalog code paths + workbench order), `frontend/styles.css` (catalog styles) | Every existing test target stays green; the new section has no API dependency; manual verification is the workbench renders actions first and the catalog page is gone |
| `forge-lifecycle-states` (follow-on, future) | Add explicit lifecycle states beyond maturity levels if a future change needs them | `src/lifecycle/`, manifest schema, `forge-project-status` | The new `lifecycle-tracked-project-view` is the only consumer; future changes can add states without re-touching the workbench |

## Sibling and Shared Architecture Reconnaissance

| Surface | Reusable code | Compatibility gap | Decision |
|---|---|---|---|
| `frontend/index.html` workbench cards | The workbench already has `wb-actions` (per-project action buttons) and `wb-workflows` (text list of available actions) | `wb-workflows` is the text-list twin of `wb-actions`; removing it is a strict simplification | **Reorder** so `wb-actions` is first; **delete** `wb-workflows` |
| `frontend/index.html` commands section | None | The catalog table is a per-command list; the per-project buttons are the actual operator surface | **Delete** the section, the nav link, and the search/filter widgets |
| `frontend/app.js` `renderCommands` / `loadCommands` | None — the catalog JSON is still fetched (the workbench needs it for `wb-actions`) | `loadCommands` stays; `renderCommands` and the filter-population helpers go | **Keep** the fetch; **delete** the rendering + filter code paths |
| `frontend/styles.css` `.command-table` / `.command-id` / `.command-path` / `.command-guidance` | None | Strict simplification | **Delete** the styles |
| `forge-web-command-catalog` spec | The JSON endpoint is the data source for the per-project workbench | The HTML page that consumed the spec is gone; the endpoint is not | **Keep** the spec; the spec's name now refers to the API surface, not the page |
| `forge-web-command-execution` and `forge-web-command-workflows` specs | These describe the workbench's per-project action buttons and confirm-and-digest | The workbench stays | **Keep** unchanged |
| `forge-web-human-dashboard` spec (already removed in `remove-api-ui`) | The HTML portal at `/ui/*` is gone; this spec was removed | None | **Already gone** |

## User Experience and Interface Impact

`UI/UX: breaking for the catalog page (intentional); transparent for
everyone who only used the workbench`.

- A user who navigates to `#commands` (or who has it
  bookmarked) sees the section is gone. The nav link is
  gone. The workbench now answers their question.
- A user who opens a project from the fleet sees the workbench
  with actions first. Maturity level, lifecycle state, and
  one-line state description are visible at the top of the
  page. The other cards (manifest, health, status, delivery,
  evidence, upgrade) are still there and unchanged.
- An operator with a project in `L0` (`draft`) sees only
  the actions that move the project forward: "Create or
  adopt" group. An operator with a project in `L2` (`working`)
  sees "Day-to-day" (upgrade, plan, etc.). An operator with
  a project in `L4` (`archived`) sees "Retire" (restore) or
  nothing.
- The catalog JSON endpoint is still reachable at
  `/v1/admin/commands` for tooling. The `forge-web-command-catalog`
  spec describes that endpoint, not a browser page.

## BFS Impact Map

- **Capabilities added:** a per-project lifecycle indicator
  + a first-class "Project actions" card on the workbench.
- **Capabilities removed:** the standalone "Command catalog
  (reference)" browser page (the `forge-web-command-catalog`
  JSON endpoint is kept).
- **Users / flows:** operators with a bookmark on
  `#commands` lose that bookmark; everyone else is unaffected.
- **Contracts / data / persistence:** the JSON API contract
  is unchanged; the registry/journal schema is unchanged.
- **Integrations / configuration:** none.
- **Callers:** the workbench's existing per-project action
  rendering is the consumer; the catalog page's consumers
  (the browser's `renderCommands` function) are removed.
- **Failure / boundary behavior:** if the catalog endpoint
  is unavailable, the workbench's actions card renders an
  empty state ("No browser-executable actions are available
  for this project yet.") instead of the old
  "Command catalog unavailable" notice on the catalog page.
- **Tests:** the surviving `forge-web-command-catalog-contract`
  and `forge-web-command-workflows-contract` test targets
  exercise the JSON endpoint and the workbench's button
  rendering; no test changes.
- **Privacy / security:** the workbench already enforces
  preview → confirm and one-plan-digest-per-batch; the new
  card is bound by the same machinery.

## Capabilities

This change **modifies** the operator-facing web UI:

- Per-project workbench: identity + lifecycle state +
  contextual action buttons first, then read-only cards
  (manifest, health, status, delivery, evidence, upgrade).
- The standalone "Command catalog (reference)" browser page
  is removed.

This change **keeps** the JSON API capability:

- `forge-web-command-catalog` (the `/v1/admin/commands`
  endpoint the workbench already calls).

## Non-goals

- No CLI change.
- No JSON API change. The `/v1/admin/commands` endpoint is
  unchanged; the workbench still calls it.
- No registry/journal schema change.
- No removal of the `forge-web-command-catalog` /
  `forge-web-command-execution` /
  `forge-web-command-workflows` specs (the JSON API and
  the per-project button rendering are still in scope; only
  the browser's reference page is removed).
- No lifecycle-state engine change in this change. The
  workbench surfaces the maturity level that already exists
  on every project record; a future change can add
  explicit lifecycle states.
- No backend change to the workbench JSON shape.
- No rename of the `/v1/admin/commands` endpoint.
