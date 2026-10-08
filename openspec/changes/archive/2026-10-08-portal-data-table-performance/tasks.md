# Tasks: Portal data table performance

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Map each requirement/scenario to its `frontend/` symbol (fleet headers, `renderProjects`, export button, pager, skeleton rows, summary-count reserve) and test assertion.
- [x] Read `frontend/index.html` project panel, `frontend/app.js` fleet render path (`renderProjects`, `fleetFilterParams`, `dashboardPage`), `frontend/styles.css` slice blocks, and `tests/forge_web_navigation_contract.rs` token assertions.
- [x] Record pre-change fleet behavior (unsorted text headers, full-mount render, text-only loading, unreserved counts) without changing live data.
- [x] Verify the implementation-handoff gate: language (vanilla HTML/CSS/JS, no build), project (vendored `frontend/`), shared-library (none — project-local), UI/UX boundary (fleet table + loading placeholders only).

## 2. DFS — Requirement-by-requirement implementation

- [x] `frontend/index.html`: fleet `Project`/`Details`/`Status` headers become `<button>` sort controls with `aria-sort` on the `<th>`; add `#fleet-export` CSV button; add `#fleet-pager` (prev/next + page info); keep the `Open` column unsorted and all ids/labels referenced by existing code intact.
- [x] `frontend/app.js`: stable sort (`fleetSort`, index tiebreak) composed after existing filters; `FLEET_PAGE_SIZE = 50` windowed render with honest `Showing X of Y` count; CSV export of filtered+sorted rows via `Blob` download (no endpoint); `renderFleetSkeleton` + skeleton loaders for delivery/maintain; `limit:1000` catalog query and JSON shape unchanged.
- [x] `frontend/styles.css`: appended slice-6 block (sort-button reset at 44px, skeleton/shimmer + reduced-motion collapse, summary-count reserve, pager layout); no earlier token changed.
- [x] `tests/forge_web_navigation_contract.rs`: new `slice_six_data_table_performance_tokens` static test (sort/export/pager/skeleton tokens + prior-slice tokens intact).
- [x] Delta spec scenarios implemented alongside the code above; payloads/endpoints unchanged.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify deep-link reload/back-forward/login behavior unchanged; slice-1 focus (`lastRouteView`, `#main-content`), scroll-padding, dark-scheme pairing, contrast pairs and slice-2 targets/touch/press/insets/dvh and slice-3 labels/summaries/toggle and slice-4 breakpoints/nav-scroll/filter-state/unknown/z-scale and slice-5 icons/type/motion untouched.
- [x] Verify no JSON API, registry/journal, catalog row/count, CLI, or static-server change; portfolio/delivery/workspace/journal tables keep their current render; no external fetch.
- [x] Verify `cargo fmt --check`, `cargo build`, `forge_web_navigation_contract` + `forge_web_manage_deep_link_browser` + `forge_web_workbench_deep_link_browser` green with actual counts.
- [x] Verify static token assertions over shipped files (sort/export/pager/skeleton + prior-slice tokens intact) and record the table.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (pre-existing warnings only).
- [x] `cargo test --test forge_web_navigation_contract` green; `cargo test --test forge_web_manage_deep_link_browser` green; `cargo test --test forge_web_workbench_deep_link_browser` green. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [x] `forge gate --dry-run`, then `forge gate --timeout-secs 600` with the verdict recorded in HANDOFF; no new failure attributable to this change.
- [x] Archived `portal-data-table-performance` via `openspec archive --yes` (no `--skip-specs`) and committed: implementation, archive+spec promotion, HANDOFF evidence in follow-up commits; `openspec list` reports no active changes.
