# Tasks: Portal focus-on-route-change and dark-theme contrast pairing

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Map each requirement/scenario to its `frontend/` symbol (`renderRoute` + `lastRouteView`, `#main-content`, `.topbar`, `--faint`, `--focus`, `.login-form input:focus`, login `color-scheme`) and test assertion.
- [x] Read `frontend/app.js` router (`VIEW_BY_PATH`/`viewForPath`/`renderRoute`/`navigateTo`/`popstate`), `frontend/index.html` shell, `frontend/login.html`, `frontend/styles.css` tokens/focus rules, and `tests/forge_web_navigation_contract.rs` token assertions.
- [x] Record pre-change contrast ratios for every `--faint`/`--muted` normal-text pair (method + values, worst pair first) without changing live data.
- [x] Verify the implementation-handoff gate: language (vanilla JS/CSS, no build), project (vendored `frontend/`), shared-library (none — project-local), UI/UX boundary (router, shell, login, tokens).

## 2. DFS — Requirement-by-requirement implementation

- [x] `frontend/app.js`: `lastRouteView` guard + programmatic `#main-content` focus at the end of `renderRoute` (view-change only, null-guarded, `preventScroll`); no change to `navigateTo`, `popstate`, param applies, or `loginNextTarget`.
- [x] `frontend/index.html`: `tabindex="-1"` on `#main-content`; nothing else.
- [x] `frontend/styles.css`: `scroll-padding-top` on `html`, sticky `.topbar`, `scroll-margin-top` on `#main-content` and titled in-view sections, keyboard ring preserved.
- [x] `frontend/login.html`: `color-scheme` `light` → `dark`; nothing else.
- [x] `frontend/styles.css`: `--faint` `#7d8698` → `#8b93a6`; define `--focus: #9aa5ff` and reference it from every `:focus-visible` rule (replacing the two dangling `var(--focus)` uses and the hardcoded value); `.login-form input:focus` keeps `border-color` and adds a 2px outline meeting 3:1.
- [x] Delta spec scenarios implemented alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify plain `/workbench`, `/management`, `login.html` behavior unchanged; hand selections never clobbered; no `#` navigation introduced; `next` guard intact (absolute/foreign/unknown/`login.html` still fall back to `index.html`).
- [x] Verify no JSON API, registry/journal, catalog row/count, CLI, or static-server change; slices 2–6 surfaces untouched.
- [x] Verify `cargo fmt --check`, `cargo build`, `forge_web_navigation_contract` + `forge_web_manage_deep_link_browser` + `forge_web_workbench_deep_link_browser` green with actual counts.
- [x] Verify post-change contrast ratios for every touched pair (all ≥4.5:1 normal text, focus ≥3:1) and record the table.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (pre-existing warnings only).
- [x] `cargo test --test forge_web_navigation_contract` green; `cargo test --test forge_web_manage_deep_link_browser` green; `cargo test --test forge_web_workbench_deep_link_browser` green. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [ ] `forge gate --dry-run`, then `forge gate --timeout-secs 600` with the verdict recorded in HANDOFF; no new failure attributable to this change.
- [ ] Archived `portal-focus-route-contrast` via `openspec archive --yes` (no `--skip-specs`) and committed: implementation, archive+spec promotion, HANDOFF evidence in follow-up commits; `openspec list` reports no active changes.
