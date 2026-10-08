# Design: Portal layout navigation

## 1. Decisions

- **Appended slice-4 CSS block (chosen).** Same convention as slices 2–3:
  one commented block at the end of `frontend/styles.css`,
  equal-specificity later rules, earlier minified rules untouched.
  Desktop above `1024px` keeps every computed value; new rules only
  tighten at or below `1024px`/`375px` or under a short-landscape gate.
  Alternative (rewriting the minified base rules) rejected: the base is
  shared by all slices and a rewrite risks regressing slices 1–3.
- **Breakpoint scale with named steps (chosen).** `1024px` (laptop:
  content padding tightening only), the existing `850px` rule documented
  as the 768-tablet step (sidebar becomes the top row), `560px`
  large-phone (kept), new `375px` small-phone (padding, type, and sidebar
  compaction), short-landscape (`orientation:landscape` with
  `max-height:500px`, so desktop landscape is excluded) compacting the
  auth hero. `1440px` needs no rule: the `.content-wrap`
  `min(1260px,100%)` cap already bounds wide viewports, recorded in the
  scale comment as the verified-unchanged wide end.
- **Scrollable sidebar nav row, not wrapping or a hamburger (chosen).**
  At `<=850px` the nav becomes `display:flex; overflow-x:auto` with
  `flex:none; white-space:nowrap` links; at `<=375px` the sidebar wraps
  brand-above/nav-below so the row never squeezes. No page-level
  horizontal scroll: every flex/grid ancestor on the path already has
  (or gains in the block) `min-width:0`, and the nav is the only
  scroller. Alternative (hamburger menu) rejected: 5 links fit a scroll
  row, and a menu would hide the active state the requirement keeps
  visible. The active link keeps `.nav-active` + `aria-current`; keyboard
  focus into it scrolls it into view natively.
- **`sessionStorage` filter snapshot, URL still owning `?project=`
  (chosen).** Eight inputs snapshotted (`project-search`,
  `source-filter`, six `filter-*`): persist on `input`/`change` and on
  `navigateTo`, restore on dashboard boot and on switching back into
  the projects view, then re-run the fleet render path
  (`refreshFleetFilters` when a predicate is active, else
  `renderProjects`). `sessionStorage` (tab-scoped, cleared with the
  session) over `localStorage` (would leak last session's filters into
  the next sign-in). The snapshot holds only typed filter strings —
  never the workbench/management selection, which is always read from
  the URL. Alternative (encoding filters into the URL) rejected: it
  would collide with the `?project=` source-of-truth contract owned by
  fleet-manage-deep-link.
- **Honest unknown section, not a redirect (chosen).** `renderRoute`
  checks `isRoutePath(pathname)` first: unknown hides every known view
  plus shows `#view-unknown` (heading, one-line explanation, the 5 real
  destination links as 44px `.button` anchors), clears all nav active
  states, sets crumb/title "Page not found". No redirect: a redirect
  would rewrite history the operator did not ask for and would fight
  the server 404 for hard loads. Unknown counts as a view for the
  `lastRouteView` focus guard, so arriving there announces via
  `#main-content` focus exactly like a known switch.
- **Token-based z-index scale (chosen).**
  `--z-sticky:10; --z-nav:30; --z-dropdown:40; --z-banner:45;
  --z-overlay:50; --z-skip:60`. Wired: skip-link 20→60 (always on top),
  topbar 10→`var(--z-sticky)` (value unchanged), sidebar
  →`var(--z-nav)` (its `position:relative` already establishes the
  context), `.panel-tools` →`var(--z-dropdown)` (select wrappers stack
  above cards), `.notice`/`.error-summary` →`var(--z-banner)`
  (position:relative added, visually neutral),
  `.wb-action-card.is-open` →`var(--z-overlay)`. No other positioned
  element exists, so the order is total and the next overlay takes the
  next free rung instead of a magic number.

## 2. Implementation boundary

- **Repository / project:** this Forge repository, vendored `frontend/`
  only. No sibling touched.
- **Files changed:**
  - `frontend/index.html` — `#view-unknown` section only (heading,
    explanation, 5 destination links, region label).
  - `frontend/app.js` — unknown branch in `renderRoute` (+
    `VIEW_CRUMBS.unknown`), filter snapshot/restore helpers
    (`readFilterState`, `persistFilterState`, `restoreFilterState`,
    `FILTER_STATE_IDS`, `FILTER_STATE_KEY`) wired into `navigateTo`,
    `renderRoute` (projects-view switch), `dashboardPage` boot, and
    the search/source/fleet input listeners.
  - `frontend/styles.css` — appended slice-4 block only (scale comment,
    z-index tokens + wiring, 1024px rule, sidebar scroll row, 375px
    rule, short-landscape rule).
  - `tests/forge_web_navigation_contract.rs` — one new static token
    test for slice 4.
  - OpenSpec package (`proposal.md`, `design.md`, `tasks.md`,
    `specs/portal-web-ui/spec.md` delta).
- **Modules reused unchanged:** `VIEW_BY_PATH`, `VIEW_IDS`,
  `normalizePath`, `isRoutePath`, `splitRoute`, `loginNextTarget`,
  `applyManagementProjectParam`, `applyWorkbenchProjectParam`,
  `lastRouteView` guard, `scrollIntoViewRespectingMotion`, all fetch
  routes and payload shapes, slices 1–3 tokens.
- **Must NOT change:** JSON API, auth/session server behavior, registry
  or journal, `command_catalog` rows, static-server allowlist, typed
  payload shapes, slices 5–6 surfaces.

## 3. Language and runtime

Vanilla JS (ES2021, no build step) in `frontend/app.js`, served by
`forge web serve` and staged by `scripts/web.sh` symlinks. Vanilla CSS
in `frontend/styles.css` (six new `--z-*` custom properties; no other
new tokens). `sessionStorage` access is try/caught (private mode,
disabled storage). Verification: Rust integration oracles
(`cargo test --test <name>`) plus static token assertions over the
shipped files; no new browser harness in this slice.

## 4. User experience and interface

Actor: any operator on a 375px phone, a landscape phone, or arriving at
an unknown dashboard path — including screen-reader and keyboard-only
operators — plus any operator whose fleet search/filter context must
survive navigation.

| Action | Before | After |
|---|---|---|
| Open any view at 375px wide | Brand + 5 nav links squeeze one row with no overflow rule; page can scroll sideways | Sidebar wraps brand-above/nav-below; nav scrolls in-row; page never scrolls sideways; active link stays visible |
| Open login/dashboard in short landscape | 380px hero + story margins push the form off-screen | Compact hero, smaller headline, two-column auth kept; form reachable in the first viewport |
| Reload or go Back with fleet search/filters typed | Typed context lost (reload) or stale | Search, source, and predicate values restored; table re-filtered; `?project=` still URL-owned |
| Open an unknown dashboard path client-side | Fleet renders under the "All projects" crumb | "Page not found" section names the miss and links the 5 real destinations; no nav link claims active |
| Layered chrome (skip/topbar/nav/dropdowns/banners) | One magic `z-index:20` | Named `--z-*` rungs; skip always topmost, sticky bar under nav, dropdown wrappers above cards |

## 5. Behavioral model

- Breakpoints: base desktop (`>1024px`) unchanged; `<=1024px` tightens
  content padding; `<=850px` sidebar becomes the scrollable top row
  (existing rule + nav scroller); `<=560px` large-phone (existing);
  `<=375px` small-phone compaction; short-landscape compacts the auth
  hero only when height is `<=500px`.
- Sidebar: `.sidebar{min-width:0}`; `nav{display:flex; overflow-x:auto;
  min-width:0}` with `flex:none` nowrap links; `<=375px` adds
  `flex-wrap:wrap` on the sidebar so the nav takes its own line. Active
  state (`.nav-active` + `aria-current`) never hidden.
- Filter state: every `input` event on the 8 controls persists the
  snapshot; `navigateTo` persists before pushing; boot restores before
  the first table render; switching into the projects view restores and
  re-renders (predicate path re-queries the catalog, search/source path
  re-renders locally). Same-view `?project=` reconciliations never
  trigger a restore (view unchanged), so deep-link focus behavior is
  intact. A denied store or missing input degrades to current behavior.
- Unknown: `isRoutePath` false → all known views hidden,
  `#view-unknown` shown, nav actives cleared, crumb/title set to the
  unknown entry, focus guard treats it as a switched-to view. Missing
  section element null-guards to the prior projects fallback.

## 6. Contract and compatibility

No wire, storage, or CLI contract changes. Browser-only deltas:
responsive rules, unknown section + branch, filter snapshot key,
z-index tokens. Backward compatible: all existing `app.js` token
assertions (`VIEW_BY_PATH`, `renderRoute`, `history.pushState`,
`popstate`, `applyManagementProjectParam`,
`applyWorkbenchProjectParam`, `loginNextTarget`, `nothing was
selected/loaded`, `match.state === "registered"`) remain literally
present; bare `/workbench`, `/management`, `login.html`, and the `next`
allowlist behave as before; slices 1–3 rendering unchanged. Server
unknown-path 404 behavior is untouched.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| `#view-unknown` missing (foreign shell) | Null-guard falls back to the prior projects default |
| `sessionStorage` denied or snapshot unparseable | Restore skipped; live input values rule; no throw |
| Restored input element missing | That id skipped; the rest still apply |
| Restore on a non-projects view | Never runs; workbench/management deep links untouched |
| Desktop landscape (`height > 500px`) | Short-landscape rules do not apply; desktop identical |
| Forced-colors / high-contrast | Scroll row uses native overflow; active state keeps text + `aria-current`, never color alone |
| Keyboard in the scroll row | Links stay tabbable; `:focus-visible` outline intact; focused link scrolls into view |

## 8. Verification oracle

- `cargo fmt --check`, `cargo build` clean (no Rust touched; confirms no
  collateral).
- `cargo test --test forge_web_navigation_contract`,
  `--test forge_web_manage_deep_link_browser`,
  `--test forge_web_workbench_deep_link_browser` green (tokens + behavior
  intact).
- Static assertions by inspection over shipped files: `1024px` and
  `375px` rules plus short-landscape gate present; desktop-above-1024
  values untouched; sidebar nav scrolls in-row with no page-level
  horizontal scroll; unknown section + branch present with cleared nav
  state; filter snapshot/restore present on all 8 controls with
  `?project=` excluded; `--z-*` scale present and wired to
  skip/topbar/sidebar/dropdown/banner/overlay; 44px minima, labels,
  summaries, focus/contrast, deep-link tokens intact.
- `node scripts/check-openspec-change-names.mjs` PASS;
  `openspec validate --all --strict --no-interactive` 0 failures;
  `git diff --check` clean.
- `forge gate --dry-run` then `forge gate --timeout-secs 600`; verdict
  recorded in HANDOFF with attribution.

## 9. Decision ledger

- **Resolved:** appended slice-4 block over base rewrite (slices 1–3
  regression risk).
- **Resolved:** scrollable nav row over hamburger (active state stays
  visible; 5 links fit).
- **Resolved:** `sessionStorage` snapshot over URL-encoded filters
  (keeps `?project=` source-of-truth; tab-scoped over persistent).
- **Resolved:** honest unknown section over redirect (no history
  rewrite; announces via the existing focus guard).
- **Resolved:** token z-scale over magic numbers (next overlay takes a
  rung).
- **Resolved:** no new browser harness; automation stays the full-audit
  concern, not this slice's.
- **Deferred (non-goals):** slices 5–6; light theme; icon/typography/
  motion tokens; table virtualize/sort/export/skeleton.
- **Blockers:** none.
