# Proposal: forge-portal-responsive-a11y

## Why

The Forge control-plane **SPA** portal (`frontend/index.html`,
`frontend/app.js`, `frontend/styles.css`) ships the same
accessibility/responsiveness contract the archived Maud-portal change
`portal-accessible-responsive-ui` (2026-10-06) established for the
server-rendered pages, but the SPA stylesheet is missing three concrete
pieces the lineage promised and operators still hit:

- No `@media (max-width: 720px)` breakpoint. The sidebar already
  collapses to a top row at `<=860px` and summary/source cards already
  auto-fit, but the workbench multi-card grid keeps two columns into
  narrow widths and the dense 8-column **Bulk Workspace onboarding**
  table (`frontend/index.html:432-433`, `#ws-rows`) never reflows — it
  only horizontally scrolls inside `.table-scroll`, hiding columns off
  the right edge on phones.
- No explicit `prefers-reduced-motion` treatment for the lifecycle rail
  (`.lifecycle-rail`), panels (`.project-panel`) and cards (`.wb-card`,
  `.summary-card`). A universal `*` guard exists at `styles.css:23`, but
  the contract is not named for these motion surfaces, and the rail
  caret / card hover / button press transitions are not explicitly
  covered by a portal-scoped rule.

Sibling operators must be able to onboard a workspace from a phone and
must not get motion they asked the OS to suppress.

## What Changes

- Add a `prefers-reduced-motion: reduce` block in `frontend/styles.css`
  that explicitly disables transitions/animations on the lifecycle rail,
  panels, cards, and the shared button/nav/action controls.
- Add a `@media (max-width: 720px)` breakpoint that: keeps the sidebar
  collapsed to a top row (inherited from the `<=860px` rule, re-asserted
  for reachability), forces the workbench multi-card grid to a single
  column, keeps the summary/source cards wrapping, and guarantees every
  control stays keyboard-reachable with the existing `>=44px` touch
  targets (slice 2) intact.
- Add a bulk-table responsive treatment: at `<=720px` the 8-column
  workspace onboarding table reflows to one stacked card per row. Column
  labels are rendered from column position via `td::before` +
  `:nth-child` so the JS-rendered `<td>` cells (`frontend/app.js:4289`)
  need **no** `data-label` attribute and `app.js` is untouched — all
  eight columns (Onboard, Directory, Project id, Action, Profile signal,
  State, Id override, Profile override) remain present and readable.

## BFS Impact Map

- **Files:** `frontend/styles.css` (add two blocks); `frontend/index.html`
  (unchanged — no `data-label` needed thanks to the `:nth-child` label
  pattern); `frontend/app.js` (unchanged — no behavioral change).
- **Selectors touched:** `.workbench-grid`, `#ws-table-wrap` (table,
  thead, tbody, tr, td), `.lifecycle-rail`, `.project-panel`, `.wb-card`,
  `.summary-card`, `.nav-link`, `.button`, `.button-quiet`,
  `.button-primary`, `.wb-action-head`, `.wb-action-caret`.
- **Contracts:** presentation-only. No API route, envelope, CLI, MCP,
  identity, persistence or publication change. No new JS behavior.
  The change extends the `control-plane-portal` "Accessible controlled
  operations" contract with an explicit responsive + reduced-motion
  rendering requirement.
- **A11y invariants preserved (must not break):** the `skip-link`
  (`frontend/index.html:15`), every `role="status"` / `aria-live`
  region (e.g. `ws-summary`, `source-list`, `ws-notice`,
  `wb-gate-status`), the `aria-label` landmarks on `sidebar` / `main`
  / `table-scroll` regions, visible `:focus-visible` outlines, and the
  `>=44px` touch-target minima from slice 2. The table keeps
  `role="region"` + `aria-label="Workspace candidates"` and its
  `<thead>` is visually hidden (not removed) so screen readers still
  announce column context.
- **Verification:** the shipped `frontend/styles.css` carries both media
  blocks and the eight `:nth-child` label rules; the shipped
  `frontend/index.html` keeps the skip link and live regions. A light
  `tests/web_portal_responsive_a11y_contract.rs` greps the shipped
  assets for these markers. No `cargo build`/`cargo test`/`forge gate`
  is run by this change (manager runs heavy checks).
- **Unaffected:** routing refactor (`405d824`), gate controls
  (`0adb844`), portfolio interest (`fdaae37`), workspace status summary
  (`0f2c828`), and all sibling project source/runtime.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `control-plane-portal`: adds the "Responsive layout and
  reduced-motion rendering" requirement (720px breakpoint, single-column
  workbench grid, stacked-card 8-column onboarding table, `>=44px`
  targets, reduced-motion disables transitions on rail/panels/cards).

## Non-goals

- Replacing the existing `<=860px` / `<=560px` / `<=375px` breakpoints
  or the universal reduced-motion guard — the new 720px block is
  additive and layered between them.
- Adding a hamburger menu or JavaScript-driven navigation — the sidebar
  already collapses to a reachable top row; this change re-asserts
  reachability only.
- Any behavioral change to `frontend/app.js`, any `src/**` backend
  change, or any new API route.
- Removing or hiding any of the eight onboarding columns.
