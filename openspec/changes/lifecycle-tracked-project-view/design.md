# Design: Lifecycle-tracked project view

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the
  `frontend/` static assets and the `forge-web-*` specs that
  describe the workbench surface. No Rust change. No
  CLI change. No JSON API change.
- **Files modified:**
  - `frontend/index.html`: reorder the workbench cards so
    "Project actions" is the first card; remove the
    "Workflows" card and the standalone "Command catalog
    (reference)" `<section id="commands">` block; remove
    the "Commands" nav link.
  - `frontend/app.js`: remove `renderCommands`,
    `loadCommands`, `populateCommandFilters`,
    `populateCommandCategories`, `showCommandsError`, the
    `command-search` / `category-filter` /
    `availability-filter` / `command-count` /
    `command-no-results` element lookups, and the
    `renderWorkflows` call from the workbench's
    `loadWorkbenchDetail` success path.
  - `frontend/styles.css`: remove `.command-table` /
    `.command-id` / `.command-path` / `.command-guidance`
    and any other styles only the removed page used.
- **Files unchanged:**
  - `forge-web-command-catalog` / `-execution` / `-workflows`
    specs.
  - The `loadCommands` JSON fetch (the workbench's action
    rendering still consumes the response).
  - The build / test / gate workflow.

## 2. Language and runtime

- The web UI is plain HTML + ES2020 JS. No build step.
- The change is a strict refactor of the browser-only
  surface; no Rust, no API, no schema change.

## 3. Ownership and shared code

The JSON endpoint `/v1/admin/commands` is the single source
of truth for the per-project action list. The workbench
already fetches it once on page load (`loadCommands`); the
fetch stays, the rendering changes.

The workbench's per-project buttons live in `#wb-actions`
and are built by `renderProjectActions()` +
`buildActionControl()`. Both stay. The only change is
*where* the resulting card sits in the workbench layout
(it moves to the top) and *what* the card's heading says
("Project actions" → "Manage this project", with a one-line
description that names the project's maturity level and
lifecycle state).

## 4. User experience and interface

- The workbench opens with a header showing
  `{project.name} · {maturity} · {lifecycle_state} —
  {one_line_state_description}`. The header is the same
  size as the page's `<h2>` so it does not introduce a
  new visual rhythm.
- The "Manage this project" card renders one button per
  available action, grouped by lifecycle stage:
  - **Create or adopt** — visible when the project is in
    `L0` (`draft` / `unregistered`).
  - **Day-to-day** — visible when the project is in `L1`
    or `L2` (working, building features).
  - **Release** — visible when the project is in `L2` or
    `L3` (ready to publish).
  - **Retire** — visible when the project is in `L3` or
    `L4` (deprecating or archived).
- The other cards (manifest, health, status, delivery,
  evidence, upgrade) move below the actions card and are
  otherwise unchanged.
- The "Commands" nav link is removed; the section it pointed
  at is removed; the page renders an empty `<main>` for
  anyone who navigates to `#commands` (the nav link's
  target).

## 5. Behavioral model

- A project in `L0` shows only the "Create or adopt" group
  of actions. The `forge-web-command-catalog` JSON endpoint
  still returns every CLI command; the workbench's
  `renderProjectActions` filters to the rows whose
  `execution.route` contains `{id}` and whose `availability`
  is `web`, then groups them by the new grouping rule.
- A project in `L2` shows the "Day-to-day" + "Release"
  groups.
- A project in `L4` shows only the "Retire" group.
- If the catalog JSON endpoint is unavailable, the actions
  card renders a single empty state ("No browser-executable
  actions are available for this project right now.")
  instead of the old per-page error.

## 6. Contract and compatibility

- JSON API contract: unchanged. `/v1/admin/commands`
  returns the same shape.
- The CLI surface: unchanged.
- The registry/journal schema: unchanged.
- The web UI's HTML output: the workbench's first
  visible card changes; the catalog page is gone.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| Catalog JSON endpoint unavailable | The workbench's actions card renders a single empty-state message; no per-page error |
| `wb-actions` has no rows | The card renders "No browser-executable actions are available for this project right now." |
| Operator navigates to `#commands` | The section is gone; the page renders no commands surface (the nav link is also gone) |
| Project's maturity level changes | The next workbench open re-fetches and re-renders the actions; the lifecycle header updates |

## 8. Verification oracle

- `cargo test --workspace` stays green (no Rust change).
- `frontend/index.html` no longer contains `id="commands"`,
  `id="command-search"`, `id="category-filter"`,
  `id="availability-filter"`, `id="commands-table"`,
  `id="command-rows"`, `id="command-count"`, or
  `id="command-no-results"`.
- `frontend/app.js` no longer defines `renderCommands`,
  `loadCommands`, `populateCommandFilters`,
  `populateCommandCategories`, or `showCommandsError`.
- `frontend/styles.css` no longer contains `.command-table`,
  `.command-id`, `.command-path`, or `.command-guidance`.
- The nav list no longer contains a "Commands" link.
- The workbench renders the "Project actions" card first
  (above manifest, health, status, delivery, evidence,
  upgrade).
- The "Workflows" text list is gone from the workbench.
- `forge-web-command-catalog` / `-execution` / `-workflows`
  spec files are still present and unchanged.
- Manual smoke: `scripts/web.sh start --projects-root <path>`
  → open the dashboard → open a project from the fleet
  → the workbench's first card shows the project's identity
  + lifecycle state + action buttons; the catalog page is
  not in the nav.

## 9. Decision ledger

- **Resolved:** the workbench is the operator's surface;
  the catalog page is gone.
- **Resolved:** the per-project action list is grouped by
  lifecycle stage, not by command name; the operator does
  not need to remember command names.
- **Resolved:** the JSON endpoint stays; the workbench is
  its only consumer.
- **Resolved:** the maturity level is shown in the
  workbench header; no new lifecycle state machine in this
  change.
- **Blockers:** none.
