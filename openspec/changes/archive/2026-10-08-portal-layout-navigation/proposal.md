# Proposal: Portal layout navigation (slice 4)

## Why

Prior UI/UX audit of `frontend/` (vanilla HTML/CSS/JS, no framework) found
six gap slices; slices 1–3 (`portal-focus-route-contrast`,
`portal-touch-responsive-targets`, `portal-form-error-feedback`) are
delivered and archived. This change implements ONLY slice 4 (responsive
layout, sidebar navigation reachability, filter state preservation,
unknown-route fallback, z-index layering). Slices 5–6 stay QUEUED and are
not authored as active changes by this package.

Measured defects in slice 4 scope (current `main`):

- Breakpoints are ad-hoc: exactly two (`850px`, `560px` in
  `frontend/styles.css:21-22`). There is no rule at `375px` (small
  phone), no `768px`/`1024px` step between phone and desktop, no wide
  guard toward `1440px`, and no landscape/short-height handling — a
  landscape phone (~740x360) pays the full `380px` hero plus `55px`
  story margins (`styles.css:21`), pushing the sign-in form out of the
  first viewport.
- At `375px` the sidebar is one row (`styles.css:21`) holding the brand
  plus 5 text-only nav links (`index.html:19`; icons hidden at
  `styles.css:22`). The row has no overflow treatment (`overflow`,
  wrapping, or scroll), so the 5 destinations can force page-level
  horizontal scrolling — the one thing the responsive contract forbids.
- The router drops operator filter/search state: `renderRoute`
  (`app.js:93`) toggles views only, and `navigateTo` (`app.js:135`)
  carries just `pathname + search`. The fleet search box
  (`#project-search`), source filter (`#source-filter`), and the six
  catalog predicate inputs keep DOM values in-session but lose them on
  reload; a back/forward traversal never restores what the operator
  typed. `?project=` deep links stay URL-owned (fleet-manage-deep-link)
  and must remain so.
- Unknown paths fall back silently: `viewForPath` (`app.js:72`)
  defaults to `"projects"`, so any pathname outside `VIEW_BY_PATH`
  renders the fleet with the "All projects" crumb — an honest
  misdirection (server-side unknown paths 404, but any client-side
  unknown pathname claims to be the fleet).
- Layering is a single magic number: the only `z-index` in the sheet is
  `.skip-link` (`z-index:20`, `styles.css:7`) versus sticky `.topbar`
  (`z-index:10`, `styles.css:11`). Sidebar nav, dropdown wrappers,
  banners, and result overlays have no layer assignment, so the next
  overlay added will stack by accident.

## What Changes

- `frontend/styles.css`: one appended slice-4 override block —
  breakpoint scale toward 375/768/1024/1440 (new `1024px` content
  tightening, new `375px` small-phone tightening, short-landscape
  compaction; the existing `850px` rule covers the 768 tablet step and
  the desktop base above `1024px` is untouched), sidebar overflow
  treatment (scrollable nav row, no page-level horizontal scroll),
  layered `--z-*` scale rewiring skip/topbar/sidebar/dropdown/banner
  layers. Desktop layout above `1024px` is byte-identical in feel.
- `frontend/index.html`: one new `#view-unknown` section (honest
  empty/nav-explained state with the 5 real destination links); no
  other markup change.
- `frontend/app.js`: `renderRoute` renders the unknown section (with
  cleared nav active state, "Page not found" crumb/title) for any
  pathname outside the route allowlist instead of the fleet; filter
  state (`project-search`, `source-filter`, six predicate inputs)
  snapshots to `sessionStorage` on input and on `navigateTo`, restored
  on boot and on switching back into the projects view; `?project=`
  stays URL-owned and is never snapshotted or restored.
- `tests/forge_web_navigation_contract.rs`: one new static token test
  covering the unknown fallback, filter-state preservation, breakpoint
  scale, sidebar scroll treatment, and z-index scale.
- No API/registry/journal/CLI/catalog change. 44px targets, visible
  labels, error summaries, focus/contrast tokens, deep-link behaviors,
  and touch/motion handling are unchanged.

## BFS Impact Map

- **Capabilities:** `portal-layout-navigation` (new, slice-scoped).
- **Users / flows:** small-phone (375px) and landscape operators;
  keyboard/screen-reader operators meeting an unknown path; operators
  whose fleet search/filter context must survive reload, back/forward,
  and view switches.
- **Contracts / data / persistence:** none. No JSON API, registry,
  journal, schema, catalog row/count, or CLI change. Browser-only
  addition: one `sessionStorage` key (`forge.filter-state.v1`) holding
  only the operator's own typed filter strings; never a project id,
  credential, path, or hash.
- **Integrations / configuration:** none. `scripts/web.sh` staging picks
  up `frontend/` unchanged.
- **Callers:** `renderRoute`, `navigateTo`, `popstate` (via
  `renderRoute`), `viewForPath`/`isRoutePath` gain the unknown branch;
  `dashboardPage` boot and the projects-view switch gain filter
  restore; `initFleetFilters` plus the search/source listeners gain
  snapshot persistence. `applyManagementProjectParam`,
  `applyWorkbenchProjectParam`, `loginNextTarget`, the
  `lastRouteView` focus guard, and the scroll helper are untouched.
- **Failure / boundary:** unknown view null-guards a missing section;
  filter restore null-guards missing inputs and a denied/unparseable
  store; landscape rules are height-gated so desktop landscape is
  unaffected; the sidebar scroll row keeps `min-height:44px` links and
  the visible active state.
- **Tests:** `forge_web_navigation_contract` (+1 token test),
  `forge_web_manage_deep_link_browser`,
  `forge_web_workbench_deep_link_browser` must stay green; small-phone/
  landscape operability verified by static token assertions over the
  shipped files (no new browser harness; markup-only claims are not
  reported as browser verification).
- **Privacy / security:** filter snapshot holds only typed filter
  strings on the operator's own machine; `?project=` selection is never
  written to storage, always read from the URL; `next` guard untouched.

## Capabilities

- `portal-layout-navigation`: breakpoints cover 375/768/1024/1440 with
  short-landscape compaction while desktop above 1024px keeps its
  feel; the 375px sidebar row keeps all 5 destinations reachable with
  no page-level horizontal scroll and a visible active state;
  filter/search state survives reload, back/forward, and view switches
  with the URL remaining the source of truth for `?project=`; unknown
  paths render an honest empty/nav-explained state instead of the
  fleet; a layered z-index scale orders skip, nav, dropdowns, banners,
  and overlays predictably.

## Non-goals

- Slices 5–6 (SVG icons, typography, motion tokens; data-table
  virtualize/sort/export/skeleton) — separate future changes.
- No light theme, no new modules, no framework or build step, no visual
  redesign beyond the scoped responsive/nav/state/fallback/layering
  tokens.
- No workbench health latency or project-retire work (HANDOFF defects
  1–2 stand).
