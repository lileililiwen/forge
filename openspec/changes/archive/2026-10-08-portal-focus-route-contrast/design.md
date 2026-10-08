# Design: Portal focus-on-route-change and dark-theme contrast pairing

## 1. Decisions

- **Focus the region, not the heading (chosen).** After a view switch,
  `renderRoute` focuses `#main-content` (`tabindex="-1"`,
  `preventScroll: true`). Focusing the region announces the new view's
  landmark to screen readers without guessing which heading is "the" title
  per view, and `preventScroll` preserves back/forward scroll restoration.
  Alternative (focus per-view `h2`) rejected: five headings, five special
  cases, same announcement value.
- **Focus only on view change (chosen).** A module-scope `lastRouteView`
  guard means same-view `?project=` reconciliations (fleet Manage rows,
  back/forward across two managed ids) never yank focus out of the control
  the operator is using. Initial boot focuses (guard starts `null`).
- **Sticky topbar + scroll padding (chosen).** `.topbar` becomes
  `position: sticky; top: 0; z-index: 10`; `html` gets
  `scroll-padding-top: 76px` (60px bar + 16px breathing room);
  `#main-content` and titled in-view sections get `scroll-margin-top`.
  Pure-CSS, no JS scroll math, honors `prefers-reduced-motion` as before.
- **Bump `--faint`, keep hue (chosen).** `#7d8698` → `#8b93a6` keeps the
  muted-slate role while lifting the worst measured pair (badge-blend
  `#202329`: 4.30:1 → 5.11:1) above 4.5:1 with margin. All `--faint`
  pairs then measure ≥5.1:1 (method: relative-luminance ratio per WCAG,
  computed locally — see §8). `--muted` already passes everywhere (≥6.2:1)
  and is unchanged.
- **Pair login to dark (chosen).** `login.html` meta becomes
  `color-scheme: dark`, matching `index.html` and the dark-only token set.
  No light token set is introduced — that would be a redesign, out of
  slice scope.
- **Define `--focus`, keep the hue (chosen).** `--focus: #9aa5ff` in
  `:root`; all three `:focus-visible` rules reference the token (8.2–8.7:1
  on dark surfaces, ≥3:1 everywhere). `.login-form input:focus` keeps
  `border-color: var(--accent)` and adds
  `outline: 2px solid var(--focus); outline-offset: 1px` (8.4:1 on the
  `#0c0f16` input background) — visible with border suppressed, with
  forced-colors, and at 200% zoom.

## 2. Implementation boundary

- **Repository / project:** this Forge repository, vendored `frontend/`
  only. No sibling touched.
- **Files changed:**
  - `frontend/app.js` — `lastRouteView` guard + focus move in
    `renderRoute` (only).
  - `frontend/index.html` — `tabindex="-1"` on `#main-content` (only).
  - `frontend/login.html` — `color-scheme` meta `light` → `dark` (only).
  - `frontend/styles.css` — `--faint` value, `--focus` token + usages,
    login-input focus rule, sticky topbar, scroll padding/margins,
    `#main-content:focus:not(:focus-visible)` outline suppression (only).
  - OpenSpec package (`proposal.md`, `design.md`, `tasks.md`,
    `specs/portal-web-ui/spec.md` delta).
- **Modules reused unchanged:** `VIEW_BY_PATH` / `viewForPath` /
  `isRoutePath` / `splitRoute` / `navigateTo` / click interceptor /
  `popstate`, `applyManagementProjectParam`, `applyWorkbenchProjectParam`,
  `loginNextTarget`, `loadWorkbenchDetail`, session endpoints, `src/web.rs`.
- **Must NOT change:** JSON API, auth/session server behavior, registry or
  journal, `command_catalog` rows, static-server allowlist, `next` guard
  logic, slices 2–6 surfaces (touch sizes, safe-area, forms overhaul,
  breakpoints, icons/typography/motion tokens, data-table features).

## 3. Language and runtime

Vanilla JS (ES2021, no build step) in `frontend/app.js`, served by
`forge web serve` and staged by `scripts/web.sh` symlinks. Vanilla CSS in
`frontend/styles.css` (no custom properties beyond the existing token
set plus `--focus`). Verification: Rust integration oracles
(`cargo test --test <name>`) plus local ratio computation; no new browser
harness in this slice.

## 4. User experience and interface

Actor: keyboard/screen-reader and low-vision operator in a desktop
browser. Entry points: sidebar nav, pasted/bookmarked deep links, login
page. Exact behavior:

| Action | Before | After |
|---|---|---|
| Sidebar nav to another view | Focus stays on the old link; SR announces nothing | Focus moves to `#main-content`; SR announces the new view landmark; URL/history identical |
| Same-view `?project=` change | (unchanged) | Focus untouched; selector/scoped-card behavior identical |
| Reload / back-forward on a deep link | View + project reconcile | Identical, plus region focus on view change only |
| Tab to a login input | Border color shifts only | Border shift + 2px `#9aa5ff` outline |
| Read badge/secondary text | Badge-blend pair 4.30:1 | ≥5.1:1 everywhere secondary text renders |
| Keyboard-tab to content below topbar | (no sticky bar) | Sticky 60px bar; focused target never slides under it |

`#main-content:focus:not(:focus-visible) { outline: none }` keeps
mouse/programmatic focus ring-free while preserving the keyboard ring.
No tab-order change (`tabindex="-1"` is skipped by Tab).

## 5. Behavioral model

- `renderRoute()`: existing view-show/chrome/param logic unchanged; at the
  end: `if (view !== lastRouteView) { lastRouteView = view; focusMain(); }`
  where `focusMain()` is `document.getElementById("main-content")` +
  `focus({ preventScroll: true })` in a null-guard (never throws when the
  element is absent, e.g. login page which has no `renderRoute` call).
- `lastRouteView` initializes `null` and lives next to `wbAutoParam` /
  `wbFleetReady` module state. `navigateTo` needs no change (it calls
  `renderRoute`). `popstate → renderRoute` unchanged.
- CSS: `html { scroll-padding-top: 76px }`; `.topbar { position: sticky;
  top: 0; z-index: 10 }`; `#main-content, section[id] {
  scroll-margin-top: 76px }` (covers the region and every titled in-view
  section without touching layout).

## 6. Contract and compatibility

No wire, storage, or CLI contract changes. Browser-only deltas:
`#main-content` focusable-by-script; topbar sticky; login declares dark;
`--faint` lighter; `--focus` defined. Backward compatible: all existing
`app.js` token assertions (`renderRoute`, `VIEW_BY_PATH`, `wbAutoParam`,
`loginNextTarget`, …) remain literally present; bare `/workbench`,
`/management`, `login.html`, and the `next` allowlist behave as before.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| `#main-content` absent (login page, future shell) | Null-guard: no throw, no focus |
| `focus()` throws (detached node in tests) | Guarded call; route render already completed |
| Reduced-motion user | No JS scroll runs at all (`preventScroll`); CSS posture untouched |
| Forced-colors / high-contrast | 2px outline uses a system-visible color token; border change retained as redundant cue |
| 320px viewport / 400% zoom | Sticky bar keeps 60px height; scroll padding is viewport-independent |
| JS disabled | Static shell renders first view as today; CSS-only fixes still apply |

## 8. Verification oracle

- `cargo fmt --check`, `cargo build` clean (no Rust touched; confirms no
  collateral).
- `cargo test --test forge_web_navigation_contract`,
  `--test forge_web_manage_deep_link_browser`,
  `--test forge_web_workbench_deep_link_browser` green (tokens + behavior
  intact).
- Local contrast audit (script-recorded ratios): every `--faint` /
  `--muted` normal-text pair ≥4.5:1 post-change (worst: badge-blend
  5.11:1); focus indicator ≥3:1 (8.2–8.7:1).
- Static assertions by inspection: `tabindex="-1"` on `#main-content`,
  `color-scheme: dark` on login, no `outline: none` without a 2px
  replacement, `scroll-padding-top` present, `--focus` defined once and
  referenced by all focus rules.
- `node scripts/check-openspec-change-names.mjs` PASS;
  `openspec validate --all --strict --no-interactive` 0 failures;
  `git diff --check` clean.
- `forge gate --dry-run` then `forge gate --timeout-secs 600`; verdict
  recorded in HANDOFF with attribution.

## 9. Decision ledger

- **Resolved:** region focus over per-view heading focus (one rule, full
  coverage, no heading inventory).
- **Resolved:** view-change gating over focus-on-every-render (protects
  in-progress confirm/select flows driven by same-view param changes).
- **Resolved:** token bump over per-site color overrides (one variable,
  every `--faint` site fixed, no selector churn).
- **Resolved:** no new browser harness; browser automation stays the
  slices-2–6 / full-audit concern, not this slice's.
- **Deferred (non-goals):** slices 2–6; light theme; icon/typography/motion
  tokens.
- **Blockers:** none.
