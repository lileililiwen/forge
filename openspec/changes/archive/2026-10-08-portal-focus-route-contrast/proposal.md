# Proposal: Portal focus-on-route-change and dark-theme contrast pairing (slice 1)

## Why

Prior UI/UX audit of `frontend/` (vanilla HTML/CSS/JS, no framework) found
six gap slices; this change implements ONLY slice 1 (P1 accessibility
focus+contrast). Slices 2–6 stay QUEUED and are not authored as active
changes by this package.

Measured defects in slice 1 scope:

- `renderRoute` (`frontend/app.js:75-102`) switches views without moving
  keyboard/screen-reader focus: after a sidebar navigation the focus stays
  on the now-hidden view's link, so screen-reader users get no announcement
  of the new view.
- No `scroll-padding-top`/`scroll-margin` exists, so a sticky topbar can
  cover the keyboard-focused target (WCAG 2.2 focus-not-obscured minimum).
- `login.html:6` declares `color-scheme: light` while `index.html` declares
  `dark` and `styles.css:6` ships dark-only tokens — the login page's theme
  declaration is unpaired with its tokens (dark-mode-pairing gap).
- `--faint: #7d8698` normal text on badge chip backgrounds
  (`rgba(255,255,255,.07)` over `--panel`, effective `#202329`) measures
  **4.30:1**, below the 4.5:1 normal-text minimum. Other `--faint` pairs
  pass but thinly (4.71:1 placeholder on `--raise`).
- `.login-form input:focus { outline: none }` (`styles.css:9`) signals
  focus by `border-color` only — no 2px perimeter indicator meeting
  focus-appearance. `--focus` is referenced twice (`:59`, `:72`) but never
  defined, so those outlines render nothing.
- Deep-link reload/back-forward behavior delivered by
  `fleet-manage-deep-link` must keep working byte-identically.

## What Changes

- `frontend/app.js` `renderRoute`: after a view switch, move focus to the
  `#main-content` region (new `lastRouteView` guard; same-view `?project=`
  reconciliations never steal focus). `popstate`/click/`navigateTo`/boot
  all flow through `renderRoute`, so deep-link reload and back/forward keep
  working unchanged.
- `frontend/index.html`: `#main-content` gains `tabindex="-1"` so it is a
  programmatic focus target without entering tab order.
- `frontend/styles.css`: `html { scroll-padding-top: 76px }`; `.topbar`
  becomes `position: sticky; top: 0` with `#main-content` and in-view
  section anchors carrying `scroll-margin-top`, so keyboard focus is never
  obscured by the bar.
- `frontend/login.html`: `color-scheme` `light` → `dark`, pairing the
  declaration with the dark-only token set both pages share.
- `frontend/styles.css`: `--faint` `#7d8698` → `#8b93a6` (badge-blend pair
  4.30:1 → 5.11:1; `--raise` placeholder 4.71:1 → 5.60:1; all other pairs
  ≥5:1); define `--focus: #9aa5ff` and use the token in every
  `:focus-visible` rule; `.login-form input:focus` keeps its border change
  AND gains a 2px `outline` (8.4:1 on the input background).
- No API/registry/journal/CLI/catalog change. Plain `/workbench`,
  `/management`, and `login.html` (including the `next` open-redirect guard)
  behave as before except the scoped fixes.

## BFS Impact Map

- **Capabilities:** `portal-focus-route-contrast` (new, slice-scoped).
- **Users / flows:** keyboard and screen-reader operators navigating the
  five dashboard views; sign-in form keyboard users; low-vision users
  reading secondary text/badges.
- **Contracts / data / persistence:** none. No JSON API, registry,
  journal, schema, catalog row/count, or CLI change.
- **Integrations / configuration:** none. `scripts/web.sh` symlink staging
  picks up `frontend/` unchanged.
- **Callers:** `renderRoute` (only new call: focus move), `navigateTo`,
  `popstate` listener, boot path; `loginPage`/`loginNextTarget` untouched;
  `loadWorkbenchDetail` `scrollIntoView` retained.
- **Failure / boundary:** focus move is guarded by view-change and wrapped
  so a missing `#main-content` never throws; sticky topbar keeps the
  existing 60px height; `prefers-reduced-motion` posture unchanged.
- **Tests:** existing `forge_web_navigation_contract` (token assertions
  retained), `forge_web_manage_deep_link_browser`,
  `forge_web_workbench_deep_link_browser` must stay green; contrast/focus
  verified by computed-ratio audit + static token assertions (no new
  harness; markup-only claims are not reported as browser verification).
- **Privacy / security:** no credential, path, or hash material enters the
  URL or notices; `next` guard untouched.

## Capabilities

- `portal-focus-route-contrast`: every dashboard view switch moves focus
  to the main content region without breaking deep links; keyboard focus is
  never obscured by the topbar; both pages declare the dark scheme their
  tokens implement; secondary/badges text meets 4.5:1; login inputs show a
  2px focus indicator.

## Non-goals

- Slices 2–6 (touch targets, safe-area/dvh/press feedback; form labels,
  error summary, password toggle; breakpoints, sidebar overflow, state
  preservation, empty route; SVG icons, typography, motion tokens;
  data-table virtualize/sort/export/skeleton) — separate future changes.
- No framework, no build step, no new modules, no visual redesign beyond
  the scoped contrast/focus tokens.
- No workbench health latency or project-retire work (HANDOFF defects 1–2
  stand).
