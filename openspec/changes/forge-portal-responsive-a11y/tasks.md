# Tasks: forge-portal-responsive-a11y

## Phase 1 — BFS baseline

- [x] Read `.ai-rules/workflow.md`, `AGENTS.md`, and the
  quality/security concerns.
- [x] Read `frontend/index.html` fully; map the skip-link, `role="status"`
  / `aria-live` regions, `aria-label` landmarks, and the 8-column
  `#ws-rows` onboarding table (lines 432-433).
- [x] Read `frontend/styles.css` fully; record existing tokens, the
  `@media` breakpoint scale (1024/860/850/560/375), the universal
  reduced-motion guard (`:23`), and the slice-2 `>=44px` minima.
- [x] Skim `frontend/app.js` `#ws-rows` renderer (`:4287-4328`) to
  confirm column order and that no `data-label` exists; decide the
  `:nth-child` label pattern so `app.js` stays untouched.
- [x] Confirm name preflight
  (`node scripts/check-openspec-change-names.mjs`) PASS.

## Phase 2 — DFS requirement implementation

- [x] Add the `@media (prefers-reduced-motion: reduce)` block disabling
  transitions/animations on `.lifecycle-rail`, `.project-panel`,
  `.wb-card`, `.summary-card`, `.nav-link`, `.button`, `.button-quiet`,
  `.button-primary`, `.wb-action-head`, `.wb-action-caret`.
- [x] Add the `@media (max-width: 720px)` block: single-column
  `.workbench-grid`; re-assert keyboard reach + `>=44px` targets; do
  not re-declare the `<=560px` summary/topbar rules.
- [x] Add the `#ws-table-wrap` stacked-card reflow at `<=720px` with
  eight `:nth-child` `::before` labels (Onboard … Profile override);
  visually hide (not remove) `<thead>`; keep checkbox + override inputs
  visible and reachable.

## Phase 3 — BFS regression / completeness

- [x] Verify the skip-link, every `role="status"`/`aria-live` region,
  `aria-label` landmarks, `:focus-visible` outlines, and slice-2
  `>=44px` minima are untouched by the new blocks.
- [x] Verify the existing 860/850/560/375 breakpoints and the
  universal reduced-motion guard are not deleted or overwritten.
- [x] Confirm `frontend/app.js` and `frontend/index.html` are unchanged.
- [x] Add `tests/web_portal_responsive_a11y_contract.rs` asserting the
  shipped `styles.css` carries both media blocks + eight `:nth-child`
  labels, and the shipped `index.html` retains the skip link and live
  regions.

## Phase 4 — Verification

- [x] `node scripts/check-openspec-change-names.mjs` → PASS.
- [x] `git diff --check` → clean.
- [x] `openspec validate forge-portal-responsive-a11y --strict --no-interactive`
  → pass.
- [x] (Heavy checks — `cargo build`, `cargo test --workspace`,
  `forge gate` — deferred to the manager.)
