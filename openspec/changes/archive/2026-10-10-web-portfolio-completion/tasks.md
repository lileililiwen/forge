# Tasks: web-portfolio-completion

## 1. BFS — Baseline and impact coverage

- [x] Confirm proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/api/portfolio.rs` (`list/detail/read_item/write_item`, `READ_KINDS`/`WRITE_ACTIONS`, `require_id`, `typed_refusal`, `scrub`), `src/api/router.rs` (portfolio arms + OPTIONS arms + exhaustiveness lists), `src/api/model.rs` (`Route::AdminPortfolio*`), `src/api/admin/deploy.rs` (portfolio dispatch + `guarded` + `is_json`), `src/api/command_catalog/routes.rs` + row table (`IMPLEMENTED_WEB_ROUTES`, count pin), `src/registry/portfolio/store.rs` (all `portfolio_*` signatures incl. `portfolio_reviews_for` cap + `SnapshotWrite` validation), `src/cli/functions_9.rs` (`cmd_portfolio_*` + `parse_evidence_status`), `frontend/index.html` (portfolio card) + `frontend/app.js` (portfolio region), and canonical `forge-web-portfolio-controls` + `portfolio-metadata-and-review` specs.
- [x] Map each requirement/scenario to handler, route arm, catalog row, card control and contract-test assertion; record toolchain (Rust 2021, `rustc 1.87` floor) and ownership (extend `src/api` + catalog + portfolio card; Core/CLI/schema unchanged).

## 2. DFS — Requirement-by-requirement implementation

- [x] `src/api/portfolio.rs` + new `src/api/portfolio_writes.rs`: extend `READ_KINDS`; add list projections (`tags|relations|reviews|goals`); confirm-gate the four existing write actions (409 + preview + none); add `remove_tag`, `remove_relation`, `import_evidence` handlers + thirteen `ROUTE_ADMIN_PORTFOLIO_*` constants; keep contract version and every existing response key. Write half split into the sibling file via `#[path]` (verbatim move) so both stay under the source-file-size cap.
- [x] `src/api/model.rs` + `router.rs` + `admin/deploy.rs` + `handlers_core.rs`: add `AdminPortfolioTagRemove|AdminPortfolioRelationRemove|AdminPortfolioEvidenceImport`, route arms, 6-segment portfolio OPTIONS arm, permission + dispatch + exhaustiveness threading; `POST …/evidence` 403 unchanged.
- [x] `src/api/command_catalog/`: fourteen `portfolio.*` leaves converted `NotYetWeb` → `web_at` + `IMPLEMENTED_WEB_ROUTES` entries; `catalog_covers_every_clap_path` count unchanged at 234 (conversion, not addition); pinned web-id vecs in `catalog.rs` + integration contract updated.
- [x] `frontend/index.html` + `frontend/app.js`: show reader, tag remove, relation add/remove/list, review list, goal add/link/list, evidence import/list — preview regions (`role=status`), confirm checkboxes, error-summary focus, `confirm: true` on every write, read-only evidence rendering, selection-preserving refreshes, no new dependency/sink.
- [x] `tests/forge_web_portfolio_controls_contract.rs`: update existing write call sites with `confirm: true`; add design §8 oracle tests (confirm refusal, remove round-trips, lists, append-only import, frontend pins, path-leak sweep).
- [x] Delta spec scenarios implemented alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Existing fleet/evidence/show shapes byte-identical except additive confirm behavior on writes; unknown-kind 404 and evidence-edit 403 unchanged.
- [x] CLI `forge portfolio *` behavior unchanged; no registry/journal schema change; reads write no rows and probe no provider.
- [x] Full forbidden-sink + inline-script + path-leak sweep over the enlarged portfolio surface (Rust + frontend).
- [x] No current-change placeholder/disabled behavior remains; `IMPLEMENTED_WEB_ROUTES` agrees with all pins; old-frontend safety (unknown ids still typed refusals).
- [x] `src/api/portfolio.rs` 724 + `src/api/portfolio_writes.rs` 406 lines, both under the source-file-size cap (gate: 398/398 pass).

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (3 pre-existing warnings + 1 pre-existing unused-import warning in `portfolio/share/validation.rs`, all untouched by this change).
- [x] `cargo test --test forge_web_portfolio_controls_contract` green (16 passed: 11 existing + 5 new); regression: lib 1213 passed, `portal_ui_contract` 21, `forge_web_navigation_contract` 9, `web_command_reference_browser_contract` 6, `forge_web_maintainer_surface_contract` 5, `forge_web_command_catalog_contract` 7 passed + 2 pre-existing cap-gap failures identical on pristine tree, `forge_web_project_workbench_contract` 10 passed + 1 pre-existing innerHTML failure, `portfolio_ui_contract` 0/20 pre-existing legacy-form failure identical on pristine tree. No new failure.
- [x] `node scripts/check-openspec-change-names.mjs` PASS; `openspec validate --all --strict --no-interactive` 0 failures; `git diff --check` clean.
- [x] Live throwaway-registry browser/curl proof: login → portfolio panel exercises every new control (show, tag add/remove, relation add/remove, review, goal save/link, evidence import) with unticked-confirm client refusal; zero attributable JS errors (only pre-existing favicon 404 + rootless workspace 409, both documented).
- [x] `forge gate --dry-run` rehearse, then full bounded `forge gate --timeout-secs 600`; verdict recorded (blocked, 0 attributable); attributable failure blocks — none.
- [ ] Archive with `openspec archive web-portfolio-completion --yes` (no `--skip-specs`); HANDOFF evidence + pointer handling; EXACTLY 2 commits (implementation+tests+package+spec, then HANDOFF only); no push.
