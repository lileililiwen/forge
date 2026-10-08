# Tasks: Portal layout navigation

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Map each requirement/scenario to its `frontend/` symbol (breakpoint/sidebar/z-index rules in `styles.css`, `#view-unknown` in `index.html`, unknown branch + filter snapshot/restore in `app.js`) and test assertion.
- [x] Read `frontend/styles.css` responsive rules, `frontend/app.js` router (`viewForPath`/`renderRoute`/`navigateTo`/`popstate`), `frontend/index.html` nav + views, and `tests/forge_web_navigation_contract.rs` token assertions.
- [x] Record pre-change responsive/nav/state/fallback/layering behavior (ad-hoc 850/560, squeezing sidebar row, reload-dropped filters, silent projects fallback, lone `z-index:20`) without changing live data.
- [x] Verify the implementation-handoff gate: language (vanilla JS/CSS, no build), project (vendored `frontend/`), shared-library (none — project-local), UI/UX boundary (breakpoints, nav, preserved state, fallback, layering).

## 2. DFS — Requirement-by-requirement implementation

- [x] `frontend/styles.css`: appended slice-4 block — breakpoint scale (1024px rule, 375px rule, short-landscape gate, scale comment covering 375/768/1024/1440); desktop above 1024px untouched.
- [x] `frontend/styles.css`: sidebar overflow treatment — scrollable nav row at <=850px, wrap at <=375px, no page-level horizontal scroll, active state visible, 44px targets kept.
- [x] `frontend/styles.css`: layered `--z-*` scale wired to skip-link, topbar, sidebar, dropdown wrappers, banners, and open action cards.
- [x] `frontend/index.html`: `#view-unknown` honest empty/nav-explained section with the 5 real destination links.
- [x] `frontend/app.js`: unknown-route branch in `renderRoute` (cleared nav state, not-found crumb/title, focus-guard participation, null-guarded section).
- [x] `frontend/app.js`: filter snapshot/restore (8 controls, persist on input + `navigateTo`, restore on boot + projects-view switch, re-render path, `?project=` excluded, store failures degrade silently).
- [x] `tests/forge_web_navigation_contract.rs`: new static token test for slice 4.
- [x] Delta spec scenarios implemented alongside the code above; payloads/endpoints unchanged.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify deep-link reload/back-forward/login behavior unchanged; slice-1 focus (`lastRouteView`, `#main-content`), scroll-padding, dark-scheme pairing, contrast (`--faint`, `--focus`) and slice-2 targets/insets/dvh/motion-guard and slice-3 labels/summaries/toggle untouched.
- [x] Verify no JSON API, registry/journal, catalog row/count, CLI, or static-server change; slices 5–6 surfaces untouched.
- [x] Verify `cargo fmt --check`, `cargo build`, `forge_web_navigation_contract` + `forge_web_manage_deep_link_browser` + `forge_web_workbench_deep_link_browser` green with actual counts.
- [x] Verify static token assertions over shipped files (breakpoints, sidebar scroll, unknown fallback, filter state, z-scale, prior-slice tokens intact) and record the table.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (pre-existing warnings only).
- [x] `cargo test --test forge_web_navigation_contract` green; `cargo test --test forge_web_manage_deep_link_browser` green; `cargo test --test forge_web_workbench_deep_link_browser` green. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [x] `forge gate --dry-run`, then `forge gate --timeout-secs 600` with the verdict recorded in HANDOFF; no new failure attributable to this change.
- [x] Archived `portal-layout-navigation` via `openspec archive --yes` (no `--skip-specs`) and committed: implementation, archive+spec promotion, HANDOFF evidence in follow-up commits; `openspec list` reports no active changes.
