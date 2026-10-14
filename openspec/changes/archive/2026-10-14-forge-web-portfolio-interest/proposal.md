# Proposal: forge-web-portfolio-interest

## Why

The CLI exposes `forge portfolio interest {import,list,show,compare,trend,audit,readiness}`
(`src/cli/commands_portfolio.rs:76-156`), and the JSON API already serves those reads under
`/v1/projects/{id}/interest`, `/v1/interest/trend`, and friends (`src/api/router.rs:474-483`,
`src/api/handlers_portfolio.rs:655-899`). The admin portal panel
(`frontend/index.html:444-532`) shows evidence, projects, tags, relations, reviews and goals — but
it surfaces **no interest reads at all**. A sibling operator who wants to see whether a project is
drawing interest has to drop to a terminal. That is the same CLI/portal gap the sibling-empowerment
effort is closing for every other portfolio surface (`forge-web-portfolio-controls`,
`forge-portfolio-ownership`).

The contract hazard is the same one the CLI already handles, and it is load-bearing: interest
evidence is **recorded, never synthesized**. A window older than `stale_after_days` reads as `stale`
in the CLI and must read as `stale` in the UI *exactly the same way* — Forge never drops it, never
refreshes it, and never presents it as the present. The projection also carries `findings`
(refusals); the UI must show those refusals, never invent a "clean" projection from a refusal.

## What Changes

- **Three read-only admin GET routes** that wrap the existing `forge::portfolio::interest_report`
  Core path (the same one the CLI and the JSON API use) — never the CLI, never a subprocess:
  - `GET /v1/admin/portfolio/{id}/interest/list` → `Route::AdminPortfolioInterestList { id }`
    — per-project snapshot list. Surfaces every stored window with its per-window `stale` label and
    a project-level `stale_label` (`no-evidence` / `current` / `stale`) derived the same way
    `project_interest` derives `stale_snapshots`.
  - `GET /v1/admin/portfolio/{id}/interest/show` → `Route::AdminPortfolioInterestShow { id }`
    — the whole projection for one project, **including `findings` (refusals)**.
  - `GET /v1/admin/portfolio/{id}/interest/trend?metric=&limit=&stale_after_days=` →
    `Route::AdminPortfolioInterestTrend { id }` — one metric's windowed history, each point already
    carrying its `freshness` (`current` / `stale`).
  - All three are `admin:access`-gated session reads. The beside-the-handler matcher lives in a new
    `src/api/admin/interest.rs` (the `agent_identity.rs` / `history.rs` precedent), registration is
    a new `Route` variant in `src/api/model.rs`, and the three classification arms are added in
    `src/api/router_classify.rs`. **Zero lines added to `src/api/router.rs`** (it stays ≤ 1000).
- **Frontend portfolio panel** (`frontend/index.html`, `frontend/app.js`): a new "Portfolio
  interest" card with a **List snapshots** and **Show projection** button (auto-loading the list when
  a project is selected) and a **Trend** control (metric select + window count). All three render via
  `textContent` only, reuse the existing `makeBadge`/`badge-stale`/`badge-fresh` chips, and show
  `stale` whenever the underlying window is stale — mirroring the CLI. `frontend/styles.css` reuses
  existing chip classes; no new dependency.
- **No write button.** The CLI `import` is correctly CLI-only; the portal adds no import control.
  `compare`, `audit` and `readiness` are out of scope for this change (they remain CLI/JSON-API
  reads; the portal surfaces only the three reads named above).

## BFS Impact Map

- **Requirements/scenarios:** `forge-web-portfolio-controls` (the portfolio panel this change hangs
  off — additive reads only; the metadata-write/evidence-ownership/preview-confirm requirements are
  unchanged and not duplicated). `forge-portfolio-ownership` (the lineage: interest evidence is
  source-owned and recorded, never synthesized — this change only *reads* it, so the ownership
  invariant is preserved, not extended). `interest-snapshots` (the Core store the reads project
  onto) — re-used, no delta.
- **Concepts/modules:** `forge::portfolio::interest_report::project_interest` and
  `interest_trend` (unchanged — the handlers delegate to them, exactly like
  `handle_get_interest` / `handle_interest_trend` in `src/api/handlers_portfolio.rs`); new
  `src/api/admin/interest.rs` (matcher + three handlers, `cors`+`guarded` envelope like
  `history.rs`/`agent_identity.rs`); three new `Route` variants; three arms in each of the three
  `router_classify.rs` matches; one new `pub mod interest;` + one `or_else` in
  `src/api/admin/mod.rs::route_beside`; three dispatch arms in `src/api/admin/deploy.rs::handle`.
- **Contracts:** `GET /v1/admin/portfolio/{id}/interest/{list|show|trend}`; success bodies reuse
  `forge::portfolio::interest::INTEREST_CONTRACT_VERSION`; the `list`/`show` envelopes inject a
  per-snapshot `freshness` (`current`/`stale`) computed by `interest::freshness_label` at read time
  (identical to the CLI's stale derivation), and a project-level `stale_label` derived from
  `stale_snapshots`; `trend` returns the `Trend` struct (points already carry `freshness`). A window
  older than `stale_after_days` reads `stale` in the UI exactly as the CLI reports it.
- **Callers/persistence:** no Core / CLI / registry-schema change. The new admin routes are the only
  new callers of `project_interest` / `interest_trend`. `project_interest` already enforces the
  managed-project gate (`registry.require_project`), so an unmanaged id returns an honest refusal.
- **Integrations:** no provider, no adapter, no shell, no browser-supplied path on any path; the
  reads are in-memory projections of stored snapshots.
- **Tests/compatibility:** new `tests/web_portfolio_interest_contract.rs` — static-token contract
  over the shipped `frontend/` assets (the new card is in `index.html`, the handlers are wired in
  `app.js`, no `innerHTML` / `eval(` / `Function(`, no write button), plus an in-process admin-route
  contract through `forge::api::handle` asserting the three reads surface and that a stale window is
  reported `stale`. **The 401/403/415 boundary is not re-tested** — `tests/forge_web_portfolio_
  controls_contract.rs` already covers the `/v1/admin/portfolio/...` session/origin boundary, and the
  new routes inherit the identical `is_admin_dispatch` → `guarded` gate; the new test asserts the
  interest-specific read surfaces only. One mutation-guard test forces the renderer to synthesize a
  non-stale label from missing data and asserts the test still fails (guards the never-synthesize
  invariant).
- **Concerns:** security (admin session cookie gate before any Core call; CORS-exact origin; no path
  in the response — `project_interest`/`interest_trend` serialize no filesystem path); quality (no
  placeholder, no new dep, no `innerHTML`, source-file-size caps); a11y (the card reuses the existing
  `wb-card` / `badge-*` / `state-*` styling and `role=status` regions); the **never-synthesize**
  invariant is preserved literally — the handlers return whatever `project_interest`/`interest_trend`
  computed, including `stale` and `findings`, and fabricate neither a `current` label from a stale
  window nor a clean projection from a refusal.

## Capabilities

- A Forge operator with a valid `forge_admin_session` can read, per managed project from the portfolio
  panel, the stored interest snapshots (with per-window stale labels), the whole interest projection
  including refusals, and one metric's windowed trend — all read through the same Core path the CLI
  uses, with stale windows labeled exactly as the CLI reports them.
- The operator can do all of the above from the browser with no write path: import stays CLI-only and
  no import control appears in the portal.

## Non-goals

- **No `import` write in the portal.** The CLI `import` is CLI-only by design; surfacing an import
  control would let the browser become a writer of interest evidence, which the ownership invariant
  forbids.
- **No `compare` / `audit` / `readiness` reads in the portal.** They remain CLI and JSON-API reads
  and are explicitly out of scope for this change.
- **No modification of `src/portfolio/**` (Core).** The reads reuse `project_interest` /
  `interest_trend` verbatim; this change is a thin admin/portal projection of existing Core behavior.
- **No new `src/api/router.rs` match arms.** All new dispatch lives in `router_classify.rs` and the
  beside-the-handler `interest.rs` module, keeping `router.rs` ≤ 1000 lines.
- **No fabrication of freshness.** A stale window is never relabeled `current`, and a refusal is
  never hidden — the UI renders the raw `stale`/`findings` the Core returns.
