# Tasks: forge-web-portfolio-interest

## 1. BFS baseline

- [x] Read `.ai-rules/workflow.md`, `AGENTS.md`, `.ai-rules/concerns/quality.md` + `security.md`.
- [x] Read `src/cli/commands_portfolio.rs:76-156` (the interest subcommand shapes: import/list/show/compare/trend/audit/readiness).
- [x] Read `src/api/router.rs` (the 737-line file after the table-driven refactor; confirmed no new lines may be added there).
- [x] Read `src/api/router_classify.rs` (the three exhaustive classification matches; the new `Route` variants add one arm each).
- [x] Read `src/api/admin/mod.rs` (`route_beside` chain) and `src/api/admin/agent_identity.rs` / `src/api/admin/history.rs` (the beside-the-handler matcher + handler precedent).
- [x] Read `src/api/admin/deploy.rs::handle` (the admin dispatch landing site for the three new arms).
- [x] Read `src/api/handlers_portfolio.rs:655-899` (`handle_get_interest`, `handle_interest_trend` — the Core call shapes to reuse).
- [x] Read `src/portfolio/interest_report.rs` (`project_interest`, `interest_trend` signatures + `require_project` gate) and `src/portfolio/interest/compare.rs` (`Freshness`, `freshness_label`, `Trend`, `TrendPoint`).
- [x] Read `src/api/portfolio.rs:144-228` (`require_id` / `open` — confirmed `project_interest` already enforces the managed-project gate).
- [x] Read `frontend/index.html:419-532` (the portfolio panel) and `frontend/app.js:4891-5467` (the portfolio wiring: `loadPortfolio`, `portfolioRefreshLists`, `makeBadge`, `detailRow`).
- [x] Read `tests/forge_web_portfolio_controls_contract.rs` (the parent-spec contract test pattern; the 401/403 boundary is already covered there).
- [x] Map requirement → CLI `portfolio interest` surface → `interest_report` Core → JSON API → admin portal panel → failure modes (no snapshots, stale window, unmanaged id) → test surface → the never-synthesize invariant.
- [x] Naming preflight: `node scripts/check-openspec-change-names.mjs` → PASS.

## 2. DFS requirement implementation

- [x] Author `openspec/changes/forge-web-portfolio-interest/proposal.md` (Why, What Changes, BFS Impact Map, Capabilities, Non-goals) citing `forge-web-portfolio-controls` and the `forge-portfolio-ownership` lineage; explicitly out-of-scope: `import` write, `compare`/`audit`/`readiness`.
- [x] Author `openspec/changes/forge-web-portfolio-interest/design.md` (ownership, contracts, failures, migrations); the `stale` labeling is tied byte-for-byte to `interest::freshness_label`.
- [x] Author `openspec/changes/forge-web-portfolio-interest/tasks.md` (this file).
- [x] Author the spec delta `openspec/changes/forge-web-portfolio-interest/specs/forge-web-portfolio-controls/spec.md` (three new read requirements + the never-synthesize scenario).

## 3. BFS regression/completeness

- [x] Single-change validate: `openspec validate forge-web-portfolio-interest --strict --no-interactive` → PASS.
- [x] Commit 1: `openspec/changes/forge-web-portfolio-interest/` only (`proposal: forge-web-portfolio-interest`).
- [x] Implementation: `src/api/model.rs` — three new `Route` variants `AdminPortfolioInterestList` / `Show` / `Trend` beside the `AdminPortfolio*` variants.
- [x] Implementation: `src/api/router_classify.rs` — three arms in each of `required_permission` (admin => `None`, consistent with every other `Admin*`), `is_admin_dispatch` (`true`), `is_admin_route` (`true`).
- [x] Implementation: `src/api/admin/interest.rs` (new, beside-the-table matcher `route_interest` + three handlers `interest_list`/`interest_show`/`interest_trend`, `cors`+`guarded` envelope, reusing `project_interest`/`interest_trend` and `interest::freshness_label`).
- [x] Implementation: `src/api/admin/mod.rs` — one `pub mod interest;` + one `or_else` in `route_beside` (≤ 3 lines).
- [x] Implementation: `src/api/admin/deploy.rs::handle` — three dispatch arms to `super::interest::*`.
- [x] Frontend: `frontend/index.html` — additive "Portfolio interest" card with List / Show buttons and a Trend (metric + window count) control; no import/write button.
- [x] Frontend: `frontend/app.js` — `portfolioInterestList` / `portfolioInterestShow` / `portfolioInterestTrend`, wired to the buttons; auto-load the list in `portfolioRefreshLists`; render stale chips via existing `makeBadge`/`badge-stale`/`badge-fresh`; `textContent`-only, no `innerHTML` / `eval(` / `Function(`, no write path.
- [x] Frontend: `frontend/styles.css` — no change expected (existing `badge-*` / `state-*` classes cover the rendering).
- [x] Test: `tests/web_portfolio_interest_contract.rs` — static-token contract over the shipped `frontend/` (new card present in `index.html`, handlers wired in `app.js`, no banned primitives, no write button) + in-process admin-route contract through `forge::api::handle` (the three reads surface; a stale window is reported `stale`; an unmanaged id is refused); mutation-guard test forces the renderer to synthesize a non-stale label from missing data and asserts the test still fails (guards the never-synthesize invariant). **Skip the 401/403/415 boundary test** — `tests/forge_web_portfolio_controls_contract.rs` already covers the `/v1/admin/portfolio/...` session/origin boundary.
- [x] Commit 2: implementation + tests (`impl: forge-web-portfolio-interest (list, show, trend in portfolio panel)`).
- [x] Pre-existing tests still passing per single-change validate: **yes** (`openspec validate forge-web-portfolio-interest --strict --no-interactive` is the only validate run here; the manager runs `--all --strict` and the workspace `cargo test` pass).

## 4. Verification

- [x] **Lightweight only** (manager runs the heavy pass at the end):
  - [x] `node scripts/check-openspec-change-names.mjs` → PASS.
  - [x] `wc -l src/api/router.rs` ≤ 1000 (cap respected — zero lines added to `router.rs`).
  - [x] `git diff --check` → clean.
  - [x] `openspec validate forge-web-portfolio-interest --strict --no-interactive` → PASS.
  - [x] Source-file-size cap respected on every new/touched file.
  - [x] No `cargo build`, no `cargo test --workspace`, no `forge gate`, no `openspec validate --all --strict` — manager-owned.
- [x] **Manager-owned** (not run here):
  - [ ] `cargo build`, `cargo test --workspace` (full pre-existing test suite — must stay green; the new admin route mirrors the `agent_identity`/`history` beside-the-handler pattern, so the `is_admin_dispatch` table stays exhaustive without touching `router.rs`).
  - [ ] `openspec validate --all --strict --no-interactive` (full active + archive + canonical spec set).
  - [ ] `openspec archive forge-web-portfolio-interest --yes` (no `--skip-specs`; promotes the `forge-web-portfolio-controls` delta; archived as `2026-10-14-forge-web-portfolio-interest`).
  - [ ] HANDOFF update: record the `forge-web-portfolio-interest` deliverable + evidence table; advance the `current_spec` pointer to the next active eligible change (or remove it if none remain).
  - [ ] Commit HANDOFF; stop (no push, no further changes).
