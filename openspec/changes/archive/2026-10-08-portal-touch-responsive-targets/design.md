# Design: Portal touch-first responsive targets

## 1. Decisions

- **One appended CSS override block (chosen).** All slice-2 style deltas
  live in a single commented `slice 2` block at the end of
  `frontend/styles.css`; the earlier minified rules are not re-wrapped.
  Equal-specificity later rules win the cascade, so each override is a
  one-line, reviewable fact. Alternative (edit every minified rule
  in place) rejected: same computed result, far noisier diff on
  2000-character lines.
- **`min-height:44px`, never `height:44px` (chosen).** Operator controls
  keep their existing padding/font-size (visual density unchanged) and
  grow only their hit area; taller content (wrapping labels, large text)
  still fits. Where a fixed `height` stood in the way (`filter-box`,
  `fleet-filter input`, `search-box`, `login-form input`, workbench text
  inputs, `#ws-rows` inputs), the override sets `height:auto` alongside
  the `min-height`. Visuals that must stay small (15px checkboxes) keep
  their size and expand the *hit area* instead: the wrapping
  `.wb-confirm` label carries `min-height:44px` (label activation toggles
  the box), and `#ws-rows` boxes render at 20px inside rows that are
  already ≥44px tall from their text inputs.
- **`touch-action: manipulation` on interactive elements (chosen).**
  `a,button,input,select,textarea,label` — removes the legacy tap delay
  without disabling pan/zoom (unlike `touch-action: none`). Scoped to
  interactives rather than `*` so scroll containers keep default gesture
  handling.
- **Opacity-only press feedback at 120ms (chosen).**
  `transition: opacity 120ms ease, background-color 120ms ease` sits
  inside the required 80–150ms window; `:active { opacity: .7 }` on
  buttons, nav links, and workbench action heads is compositor-only and
  cannot reflow layout (no padding/margin/height change on press).
  `cursor:pointer` is added to every clickable that lacked it
  (`.button-quiet`, `.nav-link`, `.wb-maintain-decide`, checkboxes).
- **Safe-area insets with zero fallbacks (chosen).** Every `env()` call
  carries `,0px`, so devices without cutouts compute today's values
  exactly. `.topbar` sticks at `top: env(safe-area-inset-top,0px)` with
  matching `padding-top` (total 60px height unchanged under
  `border-box`); `.sidebar` top padding gains the inset at base/850px/
  560px widths; the fixed `.skip-link` offsets gain top/left insets;
  `body` carries left/right/bottom insets. Both viewport metas gain
  `viewport-fit=cover`, without which iOS reports zero insets.
- **`vh` first, `dvh` second (chosen).** Each of the four `100vh` sites
  keeps its current declaration and adds the `dvh` twin immediately
  after, so browsers without dynamic-viewport support render exactly as
  today.
- **JS reduced-motion guard at the call site (chosen).** New
  `scrollIntoViewRespectingMotion(target, {block})` maps
  `matchMedia("(prefers-reduced-motion: reduce)")` to `"auto"` and
  otherwise `"smooth"`, null-guarding the target and degrading cleanly
  when `matchMedia` is absent (old embedded WebViews). Rationale: the
  existing CSS `prefers-reduced-motion` block cannot override an explicit
  JS `{ behavior: "smooth" }` option, so the check must live in JS.

## 2. Implementation boundary

- **Repository / project:** this Forge repository, vendored `frontend/`
  only. No sibling touched.
- **Files changed:**
  - `frontend/styles.css` — appended slice-2 block only (targets,
    `touch-action`, press feedback, cursor, safe areas, dvh fallbacks).
  - `frontend/index.html`, `frontend/login.html` — viewport meta gains
    `viewport-fit=cover` only.
  - `frontend/app.js` — `scrollIntoViewRespectingMotion` helper + two
    call sites (`openMaintainDecision`, `loadWorkbenchDetail`) only.
  - OpenSpec package (`proposal.md`, `design.md`, `tasks.md`,
    `specs/portal-web-ui/spec.md` delta).
- **Modules reused unchanged:** `VIEW_BY_PATH` / `viewForPath` /
  `renderRoute` / `lastRouteView` focus guard / `navigateTo` / click
  interceptor / `popstate`, `applyManagementProjectParam`,
  `applyWorkbenchProjectParam`, `loginNextTarget`, session endpoints,
  `src/web.rs`, slice-1 tokens (`--focus`, `--faint`, scroll padding).
- **Must NOT change:** JSON API, auth/session server behavior, registry
  or journal, `command_catalog` rows, static-server allowlist, `next`
  guard logic, focus-on-route-change behavior, contrast tokens, slices
  3–6 surfaces (form overhaul, breakpoints/state-preservation,
  icons/typography tokens, data-table features).

## 3. Language and runtime

Vanilla JS (ES2021, no build step) in `frontend/app.js`, served by
`forge web serve` and staged by `scripts/web.sh` symlinks. Vanilla CSS in
`frontend/styles.css` (no new custom properties; existing tokens only).
Verification: Rust integration oracles
(`cargo test --test <name>`) plus static token assertions over the
shipped files; no new browser harness in this slice.

## 4. User experience and interface

Actor: touch/coarse-pointer operator on a phone or tablet, plus
reduced-motion and notched-device users. Exact behavior:

| Action | Before | After |
|---|---|---|
| Tap any button, nav link, filter, search, login input, workbench field, fleet filter, maintain Approve/Reject | Hit area 28–42px; possible 300ms tap wait; no press state | Hit area ≥44px; immediate response; 120ms opacity dip on press; pointer cursor |
| Tick a confirm/onboard checkbox | 15px box, default cursor | Same 15px visual; label row is the 44px tap target; pointer cursor |
| Open dashboard on a notched phone / with gesture bar | Sticky topbar and sidebar can sit under the notch; content reaches screen edges | Topbar/sidebar/skip-link clear the insets; body clears left/right/bottom |
| Rotate / scroll the URL bar away on mobile | `100vh` sections jump or leave a gap | `dvh` tracks the live viewport; old browsers keep `vh` rendering |
| Maintain prefill scroll / workbench open scroll with reduced motion | Animated smooth scroll regardless of OS setting | Instant (`auto`) scroll; motion users keep `smooth` |

## 5. Behavioral model

- CSS: later-block overrides only; no selector specificity increase, no
  `!important`. `min-height` grows hit areas downward/inward without
  moving siblings (no layout shift on press: `:active` touches opacity
  alone).
- `scrollIntoViewRespectingMotion(target, options)`:
  `if (!target || typeof target.scrollIntoView !== "function") return;`
  then `behavior = reduce ? "auto" : "smooth"` with the caller's `block`
  preserved (`"center"` maintain, `"start"` workbench title).
- Viewport metas: `width=device-width, initial-scale=1` retained verbatim;
  `viewport-fit=cover` appended.

## 6. Contract and compatibility

No wire, storage, or CLI contract changes. Browser-only deltas: taller
hit areas, tap-delay removal, press opacity, safe-area padding, dvh
heights, motion-aware scroll behavior. Backward compatible: all existing
`app.js` token assertions (`renderRoute`, `VIEW_BY_PATH`,
`loginNextTarget`, …) remain literally present; bare `/workbench`,
`/management`, `login.html`, and the `next` allowlist behave as before;
slice-1 focus/contrast rendering unchanged.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| Browser without `dvh` | First (`vh`) declaration applies; rendering identical to today |
| Device without cutouts | Every `env()` falls back to `0px`; padding math equals today's values |
| `matchMedia` absent | Helper treats motion preference as unset → `"smooth"`, as today |
| Scroll target missing | Null-guard returns; surrounding render already completed |
| Keyboard-only / desktop operator | 44px targets and opacity feedback are harmless at pointer-fine widths; focus rings untouched |
| Forced-colors / high-contrast | Opacity dip stays visible; no color-only signal introduced |
| JS disabled | CSS-only fixes (targets, insets, dvh) still apply; scroll helper never runs |

## 8. Verification oracle

- `cargo fmt --check`, `cargo build` clean (no Rust touched; confirms no
  collateral).
- `cargo test --test forge_web_navigation_contract`,
  `--test forge_web_manage_deep_link_browser`,
  `--test forge_web_workbench_deep_link_browser` green (tokens + behavior
  intact).
- Static assertions by inspection over shipped files: every listed
  selector computes `min-height ≥ 44px`; `touch-action: manipulation`
  present on interactives; `:active` opacity + ≤150ms transitions
  present with no layout property in the transition list;
  `cursor:pointer` on all clickables; `env(safe-area-inset-*)` on
  topbar/sidebar/skip-link/body with `0px` fallbacks;
  `viewport-fit=cover` in both metas; zero bare `100vh` minima remain
  (each paired with a `dvh` twin); both `scrollIntoView` sites route
  through the helper and no unconditional `{ behavior: "smooth" }`
  remains.
- `node scripts/check-openspec-change-names.mjs` PASS;
  `openspec validate --all --strict --no-interactive` 0 failures;
  `git diff --check` clean.
- `forge gate --dry-run` then `forge gate --timeout-secs 600`; verdict
  recorded in HANDOFF with attribution.

## 9. Decision ledger

- **Resolved:** appended override block over in-place minified edits
  (equal computed result, reviewable diff).
- **Resolved:** `min-height` over `height` (taller content still fits;
  density preserved via untouched padding).
- **Resolved:** label-carried hit areas over enlarged checkbox visuals
  (native control semantics and table density preserved).
- **Resolved:** JS call-site motion guard over CSS-only reliance (explicit
  JS `behavior` beats the stylesheet guard).
- **Resolved:** no new browser harness; browser automation stays the
  slices-3–6 / full-audit concern, not this slice's.
- **Deferred (non-goals):** slices 3–6; light theme; icon/typography/
  motion tokens.
- **Blockers:** none.
