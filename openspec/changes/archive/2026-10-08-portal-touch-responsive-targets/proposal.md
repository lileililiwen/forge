# Proposal: Portal touch-first responsive targets (slice 2)

## Why

Prior UI/UX audit of `frontend/` (vanilla HTML/CSS/JS, no framework) found
six gap slices; slice 1 (`portal-focus-route-contrast`: focus-on-route-change,
scroll-padding, dark-scheme pairing, contrast, focus outline) is delivered and
archived. This change implements ONLY slice 2 (touch/manipulation
responsiveness). Slices 3–6 stay QUEUED and are not authored as active
changes by this package.

Measured defects in slice 2 scope (current `main`):

- Operator controls render below the 44px touch height: `.button-quiet`
  `min-height:32px`, `.button` `36px`, `.button-primary` `42px`,
  `.filter-box` `height:32px`, `.fleet-filter input` `height:32px`,
  `#ws-rows input[type=text]` `height:30px`, `.wb-field` /
  `.wb-action-card` text inputs `height:34px`, `.search-box` `height:38px`,
  `.login-form input` `height:42px`, `.wb-maintain-decide`
  `min-height:28px`, `.nav-link` with no `min-height` at all
  (`frontend/styles.css`).
- No `touch-action` declaration anywhere, so mobile browsers may apply the
  legacy ~300ms tap delay / double-tap-zoom wait on every control.
- No `:active` press feedback on `.button*` / `.nav-link` /
  `.wb-action-head`, and `cursor:pointer` is missing on `.button-quiet`,
  `.nav-link`, `.wb-maintain-decide` and checkbox inputs.
- Fixed/sticky chrome ignores display cutouts: `.topbar` (`position:sticky;
  top:0`), `.sidebar`, and the fixed `.skip-link` use no
  `env(safe-area-inset-*)`, and neither viewport meta carries
  `viewport-fit=cover` (without it the insets stay zero on iOS).
- Viewport-filling heights use the legacy unit only: `body`
  `min-height:100vh` (`styles.css:7`), `.auth-layout`
  `min-height:calc(100vh - 52px)` (`:9`), `.app-shell`
  `min-height:calc(100vh - 42px)` (`:11`), `.auth-layout` again inside the
  560px query (`:22`) — the mobile URL bar makes `100vh` wrong there.
- Both programmatic smooth scrolls — `openMaintainDecision`
  (`frontend/app.js:845`, `block:"center"`) and `loadWorkbenchDetail`
  (`app.js:1000`, `block:"start"`) — pass `{ behavior: "smooth" }`
  unconditionally. The existing
  `@media(prefers-reduced-motion:reduce)` CSS guard cannot override an
  explicit JS `behavior` option, so reduced-motion users still get animated
  scrolling from these two call sites.

## What Changes

- `frontend/styles.css` (one appended override block; earlier minified
  rules untouched):
  - Every operator text/button control gets `min-height:44px`
    (`height:auto` where a fixed `height` stood in the way):
    `.button`, `.button-quiet`, `.button-primary`, `.nav-link`,
    `.filter-box`, `.fleet-filter input`, `.search-box`, `.login-form
    input`, `.wb-field` / `.wb-action-card` text inputs, `#ws-rows`
    text inputs, `.wb-maintain-decide`. `min-height` (not `height`) keeps
    taller content fitting; visual density stays in the existing
    padding/font-size, which is unchanged.
  - Checkboxes keep their small visual and gain `cursor:pointer`; their
    wrapping `.wb-confirm` labels carry the 44px hit area (label activation
    toggles the box), and `#ws-rows` boxes grow to 20px inside table rows
    that are already ≥44px tall.
  - `touch-action: manipulation` on
    `a,button,input,select,textarea,label` removes the tap delay.
  - Press feedback: `transition: opacity/background-color 120ms ease`
    (inside the required 80–150ms window) plus opacity-only `:active`
    states — compositor-cheap, never reflows layout. `cursor:pointer`
    added everywhere clickable that lacked it.
  - Safe areas: `.topbar` sticks at `env(safe-area-inset-top)` with
    matching `padding-top`; `.sidebar` top padding gains the inset at all
    three breakpoints; fixed `.skip-link` offsets gain top/left insets;
    `body` carries left/right/bottom insets.
  - `100vh` → `100dvh` with the `vh` value kept as the first declaration
    (older-browser fallback) at all four sites.
- `frontend/index.html`, `frontend/login.html`: viewport meta gains
  `viewport-fit=cover` so the safe-area env values are non-zero on iOS.
- `frontend/app.js`: new `scrollIntoViewRespectingMotion(target, options)`
  helper (`matchMedia("(prefers-reduced-motion: reduce)")` → `"auto"`,
  else `"smooth"`; null-guard) used by both scroll call sites; no other
  behavior change.
- No API/registry/journal/CLI/catalog change. Deep-link
  reload/back-forward/login behavior and slice-1 focus/contrast behavior
  are byte-identical except through the scoped fixes.

## BFS Impact Map

- **Capabilities:** `portal-touch-responsive-targets` (new, slice-scoped).
- **Users / flows:** touch and coarse-pointer operators on phones/tablets;
  reduced-motion users triggering the two programmatic scrolls; notched /
  gesture-bar devices showing the sticky topbar or sidebar.
- **Contracts / data / persistence:** none. No JSON API, registry,
  journal, schema, catalog row/count, or CLI change.
- **Integrations / configuration:** none. `scripts/web.sh` symlink staging
  picks up `frontend/` unchanged.
- **Callers:** `openMaintainDecision` and `loadWorkbenchDetail` route
  their scroll through the new helper; `renderRoute`, `navigateTo`,
  `popstate`, param applies, `loginNextTarget`, focus guard untouched.
- **Failure / boundary:** helper null-guards a missing target and degrades
  when `matchMedia` is absent; `dvh` always has a `vh` fallback;
  safe-area `env()` always has a `0px` fallback; `:active` opacity cannot
  shift layout; checkboxes keep native semantics.
- **Tests:** existing `forge_web_navigation_contract` (token assertions
  retained — `renderRoute` still present), `forge_web_manage_deep_link_browser`,
  `forge_web_workbench_deep_link_browser` must stay green; touch/safe-area/
  motion verified by static token assertions over the shipped files (no new
  harness; markup-only claims are not reported as browser verification).
- **Privacy / security:** no credential, path, or hash material enters the
  URL or notices; `next` guard untouched.

## Capabilities

- `portal-touch-responsive-targets`: every operator control presents a
  ≥44px touch target with tap-delay-free, press-confirmed interaction;
  sticky/fixed chrome clears notches and gesture bars; viewport heights
  track the dynamic mobile viewport; programmatic smooth scroll honors
  reduced-motion.

## Non-goals

- Slices 3–6 (form labels, error summary, password toggle; breakpoints,
  sidebar overflow, state preservation, empty route; SVG icons,
  typography, motion tokens; data-table virtualize/sort/export/skeleton)
  — separate future changes.
- No light theme, no new modules, no framework or build step, no visual
  redesign beyond the scoped target/feedback/inset tokens.
- No workbench health latency or project-retire work (HANDOFF defects 1–2
  stand).
