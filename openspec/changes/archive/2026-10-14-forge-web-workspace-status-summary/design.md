# Design: forge-web-workspace-status-summary

## Ownership

The browser owns the aggregation entirely. No Core, no `src/api` handler, no
new `Route` variant is introduced. The summary reads the **existing**
`GET /v1/admin/workspace/candidates` route — the same route the bulk onboarding
panel (`wsDiscover`, `frontend/app.js:4185`) already consumes — and computes
three scalars in a new `loadWorkspaceSummary()` function in `frontend/app.js`.
This is the established sibling-read pattern (the `forge-web-workspace-onboarding`
panel, the `ws-fleet-hint`, and the SPA list-read convention in
`forge-workspace-sync`), applied one level up into the topbar chrome.

## Contracts

- **Read-only `GET /v1/admin/workspace/candidates?limit=1000`.** No new route.
  The only deviation from the onboarding loader is a raised `limit` so the full
  sibling set is returned in one page (the onboarding loader uses `limit=100`).
  The response shape is unchanged:
  `{ "candidates": [ { "directory", "state", "selectable", ... } ], "total", "cursor", "limit" }`.
- **Aggregation (pure, extracted for testability):**
  - `registered = candidates.filter(c => c.state === "registered").length`
  - `onboardable = candidates.filter(c => c.selectable === true).length`
  - `lastSync` = `Date.now()` at successful load; rendered as a relative label
    with a `stale` flag once an idle threshold elapses.
  The predicates `state === "registered"` and `selectable === true` are exactly
  those the onboarding surface already relies on (`renderWsFleetHint`,
  `frontend/app.js:4172`; `wsDiscover`, `frontend/app.js:4248`), so the summary
  and the table can never disagree.
- **No request body, no new query surface beyond `limit`, no new path.** The
  loader reuses the existing `request()` helper, so the global admin-session gate
  and exact-origin CORS already apply.

## Failures

- **`409 admin-prerequisite` (root not configured):** the route already returns
  this when `FORGE_ADMIN_PROJECTS_ROOT` is unset. The loader hides the three
  counters and renders a single `Workspace not configured` chip using the
  existing `badge-unconfigured` style. This is the *same* honest empty state the
  onboarding flow shows (`frontend/app.js:4198-4200`) — no fabricated count, no
  fallback to a guessed zero.
- **Other transport/auth error:** counters stay hidden; a `role="status"`
  `muted` note ("Workspace status unavailable") replaces them. The existing
  dashboard error path (`showDashboardError`) is untouched.
- **Empty / unparseable payload:** `registered` and `onboardable` are `0` only
  when the array is genuinely empty (a real, honest zero — not a guessed one),
  and the counters render `0` rather than `—`.
- **Stale Last-sync:** after a bounded idle period (e.g. the tab has not
  re-read the workspace) the Last-sync chip switches to the `stale` label,
  mirroring how the portal already labels stale observations elsewhere.

## Migrations

None. No schema, no route, no persisted state, no stored config. The summary is
a pure read-side projection recomputed on every dashboard load.

## Compatibility

- `src/api/router.rs` is **not** modified; it stays at 737 lines (well under the
  1000 cap). No `Route` variant, no `router_classify.rs` arm.
- `frontend/styles.css` needs at most one small rule for the topbar chip group;
  existing `.badge-*` / `.muted` / `.topbar` classes carry the rendering.
- The change is additive to the topbar and is inert on the management,
  workbench, and portfolio views.

## Verification

- `openspec validate forge-web-workspace-status-summary --strict --no-interactive`.
- New `tests/web_workspace_status_summary_contract.rs` asserts the summary
  region is present, the loader is wired with no banned primitives and no write
  button, and the extracted `aggregateWorkspaceSummary(candidates)` helper yields
  the correct `registered`/`onboardable` over a fixture. The `409` boundary and
  401/403/415 behaviors are **not** re-tested — they are already covered by
  `tests/forge_web_workspace_onboarding_contract.rs`.
