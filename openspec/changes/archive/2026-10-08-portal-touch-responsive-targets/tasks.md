# Tasks: Portal touch-first responsive targets

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Map each requirement/scenario to its `frontend/` symbol (listed selectors in `styles.css`, viewport metas in `index.html`/`login.html`, `scrollIntoViewRespectingMotion` + two call sites in `app.js`) and test assertion.
- [x] Read `frontend/app.js` scroll sites (`openMaintainDecision`, `loadWorkbenchDetail`), `frontend/index.html` / `frontend/login.html` viewport metas, `frontend/styles.css` target/height/viewport rules, and `tests/forge_web_navigation_contract.rs` token assertions.
- [x] Record pre-change target heights for every listed control (32/36/42/28/30/34/38px, nav-link none) without changing live data.
- [x] Verify the implementation-handoff gate: language (vanilla JS/CSS, no build), project (vendored `frontend/`), shared-library (none — project-local), UI/UX boundary (controls, chrome, viewport, scroll).

## 2. DFS — Requirement-by-requirement implementation

- [x] `frontend/styles.css`: appended slice-2 block — `min-height:44px` (+`height:auto` where fixed) for `.button`, `.button-quiet`, `.button-primary`, `.nav-link`, `.filter-box`, `.fleet-filter input`, `.search-box`, `.login-form input`, `.wb-field`/`.wb-action-card` text inputs, `#ws-rows` text inputs, `.wb-maintain-decide`; `.wb-confirm` label `min-height:44px`; `#ws-rows` checkboxes 20px.
- [x] `frontend/styles.css`: `touch-action: manipulation` on `a,button,input,select,textarea,label`; 120ms opacity/background transitions + opacity-only `:active` on buttons/nav-link/action-head; `cursor:pointer` on every clickable that lacked it.
- [x] `frontend/styles.css` + `index.html` + `login.html`: `env(safe-area-inset-*)` (zero fallbacks) on topbar/sidebar/skip-link/body; `viewport-fit=cover` in both viewport metas; `dvh` twin after every `100vh` minimum.
- [x] `frontend/app.js`: `scrollIntoViewRespectingMotion` helper honoring `prefers-reduced-motion`, wired into `openMaintainDecision` and `loadWorkbenchDetail`; no unconditional `{ behavior: "smooth" }` remains.
- [x] Delta spec scenarios implemented alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify deep-link reload/back-forward/login behavior unchanged; slice-1 focus (`lastRouteView`, `#main-content`), scroll-padding, dark-scheme pairing, contrast (`--faint`, `--focus`) and login focus outline untouched.
- [x] Verify no JSON API, registry/journal, catalog row/count, CLI, or static-server change; slices 3–6 surfaces untouched.
- [x] Verify `cargo fmt --check`, `cargo build`, `forge_web_navigation_contract` + `forge_web_manage_deep_link_browser` + `forge_web_workbench_deep_link_browser` green with actual counts.
- [x] Verify static token assertions over shipped files (targets ≥44px, `touch-action`, press timing, cursor, safe-area fallbacks, `viewport-fit`, dvh twins, motion-guarded scrolls) and record the table.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (pre-existing warnings only).
- [x] `cargo test --test forge_web_navigation_contract` green; `cargo test --test forge_web_manage_deep_link_browser` green; `cargo test --test forge_web_workbench_deep_link_browser` green. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [x] `forge gate --dry-run`, then `forge gate --timeout-secs 600` with the verdict recorded in HANDOFF; no new failure attributable to this change.
- [x] Archived `portal-touch-responsive-targets` via `openspec archive --yes` (no `--skip-specs`) and committed: implementation, archive+spec promotion, HANDOFF evidence in follow-up commits; `openspec list` reports no active changes.
