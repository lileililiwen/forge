# Tasks: Portal form error feedback

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Map each requirement/scenario to its `frontend/` symbol (scoped inputs in `index.html`, toggle + summary in `login.html`, `renderErrorSummary`/`setFieldError` + call sites in `app.js`, slice-3 block in `styles.css`) and test assertion.
- [x] Read `frontend/index.html` filter/delivery/portfolio controls, `frontend/login.html` form, `frontend/app.js` submit handlers (`loginPage`, portfolio, delivery, `buildActionControl`, workspace/management), `frontend/styles.css` label/error rules, and `tests/forge_web_navigation_contract.rs` token assertions.
- [x] Record pre-change label/error behavior for every scoped control (placeholder-only + `sr-only`, flat notices, `role="status"`, no toggle) without changing live data.
- [x] Verify the implementation-handoff gate: language (vanilla JS/CSS, no build), project (vendored `frontend/`), shared-library (none — project-local), UI/UX boundary (labels, summaries, toggle).

## 2. DFS — Requirement-by-requirement implementation

- [x] `frontend/index.html`: fleet filter row visible stacked labels + shared hint; `fleet-filter-error` `role="status"` → `role="alert"`; inputs wired with `aria-describedby`.
- [x] `frontend/index.html`: delivery + portfolio inputs relabelled to visible labels with helper text, inline error nodes, and per-area error-summary containers.
- [x] `frontend/login.html`: error summary container, password show/hide toggle, visible required indicators + legend; `name`/`autocomplete` unchanged.
- [x] `frontend/app.js`: `renderErrorSummary`/`setFieldError`/`clearFieldError` helpers wired into login, portfolio, delivery, `buildActionControl`, and workspace/management preview/run paths with focus moved to the summary.
- [x] `frontend/styles.css`: appended slice-3 block — visible labels, hints, inline errors, error summary, invalid outline, toggle layout; 44px minima re-asserted.
- [x] Delta spec scenarios implemented alongside the code above; payloads/endpoints unchanged.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify deep-link reload/back-forward/login behavior unchanged; slice-1 focus (`lastRouteView`, `#main-content`), scroll-padding, dark-scheme pairing, contrast (`--faint`, `--focus`) and slice-2 targets/insets/dvh/motion-guard untouched.
- [x] Verify no JSON API, registry/journal, catalog row/count, CLI, or static-server change; slices 4–6 surfaces untouched.
- [x] Verify `cargo fmt --check`, `cargo build`, `forge_web_navigation_contract` + `forge_web_manage_deep_link_browser` + `forge_web_workbench_deep_link_browser` green with actual counts.
- [x] Verify static token assertions over shipped files (visible labels, hints, summaries, alert role, toggle, autocomplete intact, no new fetch route) and record the table.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (pre-existing warnings only).
- [x] `cargo test --test forge_web_navigation_contract` green; `cargo test --test forge_web_manage_deep_link_browser` green; `cargo test --test forge_web_workbench_deep_link_browser` green. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [x] `forge gate --dry-run`, then `forge gate --timeout-secs 600` with the verdict recorded in HANDOFF; no new failure attributable to this change.
- [x] Archived `portal-form-error-feedback` via `openspec archive --yes` (no `--skip-specs`) and committed: implementation, archive+spec promotion, HANDOFF evidence in follow-up commits; `openspec list` reports no active changes.
