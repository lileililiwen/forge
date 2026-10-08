# Proposal: Portal data table performance (slice 6, LAST)

## Why

Prior UI/UX audit of `frontend/` (vanilla HTML/CSS/JS, no framework) found
six gap slices; slices 1–5 (`portal-focus-route-contrast`,
`portal-touch-responsive-targets`, `portal-form-error-feedback`,
`portal-layout-navigation`, `portal-icon-type-motion`) are delivered and
archived. This change implements ONLY slice 6 (fleet data-table
usability + render performance). The queue is empty after this slice.

Measured defects in slice 6 scope (current `main`):

- Fleet table (`frontend/index.html:79-81`, `frontend/app.js:423-490`):
  headers are plain text (`Project`, `Details`, `Status`, empty Open
  column) — not sortable, not keyboard-reachable, no `aria-sort`. The
  only ordering is API return order; an operator with dozens of rows
  cannot sort by name, profile, or status.
- No export path: the currently filtered set (free-text + source +
  catalog predicates, `app.js:423-434,520-528`) exists only as DOM rows.
  There is no way to take it elsewhere; adding a server endpoint for
  this would be new API surface for a pure projection.
- Unbounded initial render: `renderProjects` mounts one `<tr>` per
  filtered row with no cap. The catalog query already pages at
  `limit:1000` (`fleetFilterParams`, `app.js:526`), so a large fleet
  mounts up to 1000 rows at once — slow first paint, large DOM, no
  honest "showing X of Y" count (only `N of M projects`).
- Text-only loading: `#project-count` reads `Loading your projects…`
  (`index.html:51`) with an empty `<tbody>` until the fetch resolves;
  delivery (`app.js:793` — `Loading delivery status…`) and maintain
  (`app.js:826` — `Loading maintainer data…`) are bare text nodes too.
  Summary counts (`#summary-*`, `#fleet-*`) render `—` with no reserved
  space, so values pop layout (CLS) when they arrive.

## What Changes

- `frontend/index.html`: the three meaningful fleet headers become
  keyboard-reachable sort buttons (`Project`, `Details`, `Status`; the
  `Open` action column stays unsorted with its `sr-only` label);
  `aria-sort` lives on each `<th>` (`none` default). New `Export
  filtered CSV` button and a pager region (Prev/Next + honest
  `showing X of Y` count) in the project panel.
- `frontend/app.js`: stable sort (`{key, dir}`, index tiebreak) applied
  after the existing free-text/source/catalog-predicate filter pipeline
  (filter code untouched, `limit:1000` query and JSON shape unchanged);
  page/window rendering (`FLEET_PAGE_SIZE = 50`, never more than one
  page mounted); CSV download of the currently filtered+sorted rows via
  `Blob` + temp anchor (no endpoint, no server state); skeleton rows
  while the fleet fetch is in flight.
- `frontend/styles.css`: one appended slice-6 block (sort-button reset
  at 44px minima, skeleton/shimmer + `prefers-reduced-motion` collapse,
  summary-count space reservation, pager layout). No earlier token
  touched.
- `tests/forge_web_navigation_contract.rs`: one static token test for
  slice 6 (sort/export/pager/skeleton tokens + prior-slice tokens
  intact).

## BFS Impact Map

- Requirements/scenarios: 4 new `portal-web-ui` requirements
  (sortable-table, export-option, paginated render, progressive-loading
  + CLS reserve), each with observable pass/fail scenarios.
- Concepts/modules: fleet render path only (`renderProjects` +
  `dashboardPage` boot); workbench, portfolio, delivery, management,
  onboarding, router, auth, deep links untouched.
- Contracts/callers: no API/registry/journal/CLI/catalog change — the
  catalog `limit:1000` query and JSON shape stay byte-identical; the
  CSV is a client-side projection of rows already in memory.
- Persistence: none (sort/page are in-memory view state; slice-4
  `sessionStorage` filter snapshot format unchanged and still restored
  before first render).
- Integrations/tests: `forge_web_navigation_contract` (+1 static test);
  manage/workbench deep-link browser suites re-run for regression.
- Compatibility: vanilla JS/CSS, no dependency, no network fetch, no
  framework; keyboard, screen-reader (`aria-sort`, labelled pager/
  export), reduced-motion, 44px-target, responsive behaviors preserved.
- Concerns: quality (filter composition exactness, stable sort,
  honest counts, CSV escaping); accessibility (sortable-table,
  export-option, progressive-loading contracts of `portal-web-ui`).

## Capabilities

- Fleet/project tables expose sortable columns with `aria-sort`
  indicating current sort state (sortable-table).
- Operators can export the currently filtered rows as CSV through a
  plain browser download (export-option).
- Large fleets render windowed/paginated with honest counts
  (virtualize-lists intent: never mount 1000 rows at once).
- Async regions show skeleton/shimmer placeholders and reserve space
  for summary counts (progressive-loading, content-jumping/CLS).

## Non-goals

- No API/registry/journal/CLI/catalog change; no new endpoint; no
  server state; portfolio/delivery/workspace/journal tables keep their
  current unsorted full render; no virtual-scroll library; no filter
  semantics change; slices 1–5 behaviors (44px targets, labels, error
  summaries, responsive/unknown-route, icons/type/motion,
  focus/contrast, deep links) unchanged.
