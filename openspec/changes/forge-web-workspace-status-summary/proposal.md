# Proposal: forge-web-workspace-status-summary

## Why

The CLI exposes a workspace-level overview of sibling directories — how many
are registered in the registry, how many are still onboardable, and a freshness
signal — through `forge workspace` (`src/cli/setup.rs:27-35`, `cmd_workspace` →
`cmd_workspace_sync`, whose report counts registered / skipped / failed over the
configured root). The admin portal shows the same sibling data two layers down:
the bulk **Workspace onboarding** panel reads `GET /v1/admin/workspace/candidates`
and renders the per-directory table (`frontend/app.js:4185-4258`,
`wsDiscover` / `renderWsFleetHint`), but there is **no top-level workspace
counter** anywhere in the portal chrome. A sibling operator lands on "All
projects" and sees per-project rows, yet cannot see at a glance whether the
workspace root has siblings waiting to be onboarded. That is the same
CLI/portal gap the sibling-empowerment effort is closing for every other
surface (`forge-web-workspace-onboarding`, `forge-web-portfolio-controls`,
`forge-web-gate-controls`).

The contract hazard is the same one `forge-web-workspace-onboarding` already
handles and it is load-bearing: when `FORGE_ADMIN_PROJECTS_ROOT` is unset the
candidate route refuses with a typed `409 admin-prerequisite` and the portal
must surface that honestly as "no workspace configured" — **never a fabricated
count**. The same honest-empty-state invariant governs this change.

## What Changes

- **Topbar "Workspace" summary** (read-only, three compact counters): a small
  chip group added to the dashboard topbar (`frontend/index.html:24-30`) that
  mirrors `forge workspace status` — **Registered**, **Onboardable**, and
  **Last sync**. The counts aggregate from the **existing**
  `GET /v1/admin/workspace/candidates?limit=1000` route
  (`src/api/router.rs:121-123`, `Route::AdminWorkspaceCandidates`); no new
  `src/api` route is added and no Core capability is introduced.
  - `Registered` = candidates whose `state === "registered"`.
  - `Onboardable` = candidates whose `selectable === true` (the same predicate
    `renderWsFleetHint` already uses, `frontend/app.js:4172`).
  - `Last sync` = the browser-side observation time of this summary load; it
    renders a relative "synced Nm ago" label and switches to a **stale** label
    after a bounded idle threshold (the candidate route carries no server
    timestamp, so the honest signal is "when this client last read the
    workspace", exactly as every other panel shows an "Updated HH:MM" line).
- **No-write control.** The summary is a read-only mirror; it adds no onboard
  button, no link that mutates, and no new route. It is a sibling of the
  existing summary cards, not an action surface.
- **Honest empty state.** When the candidate route returns `409` (root
  unconfigured) the three counters are hidden and a single `Workspace not
  configured` chip (reusing the existing `badge-unconfigured` style) is shown in
  their place — the same condition the onboarding flow already surfaces.
- `frontend/styles.css`: no new file; the summary reuses existing chip/badge
  classes (`.badge-*`, `.muted`, `.topbar`). A tiny `.ws-summary` / `.ws-summary-chip`
  rule may be added only if the topbar layout needs it.

## BFS Impact Map

- **Requirements/scenarios:** `forge-web-workspace-onboarding` (the candidate
  route and its `409` empty-state are re-used, not re-specified — the
  discovery requirement there is the authority for both the aggregation source
  and the honest-empty-state boundary); `control-plane-portal` (the topbar is a
  read-only control-plane view; the "stale/unknown observation is labeled"
  scenario applies directly to the Last-sync chip). `forge-workspace-sync` (the
  CLI-side sibling report this mirrors) — re-used, no delta.
- **Concepts/modules:** `GET /v1/admin/workspace/candidates` response shape
  (`{ candidates: [{ directory, state, selectable, ... }], total, cursor, limit }`,
  `src/api/workspace.rs:258-325`) — the single aggregation source. No `src/api`
  module, no `Route` variant, no `router_classify.rs` arm is added; `router.rs`
  is untouched and stays ≤ 1000 lines (737 today). The browser aggregates in a
  new `loadWorkspaceSummary()` in `frontend/app.js`, wired from the dashboard
  boot (`frontend/app.js:6640-6677`, beside the existing `wsDiscover()` call).
- **Contracts:** read-only `GET`; the same `AdminWorkspaceCandidates` contract
  the onboarding panel already depends on. No request body, no new query
  surface (only a raised `limit` to capture the full sibling set). Success body
  is the existing candidate envelope; the summary reads `state` / `selectable`
  / `total` and nothing path-shaped.
- **Callers/persistence:** no Core / CLI / registry-schema change. The only new
  caller of `AdminWorkspaceCandidates` is the browser summary loader.
- **Integrations:** no provider, no adapter, no shell, no browser-supplied path;
  the loader passes only a numeric `limit` query.
- **Tests/compatibility:** the `AdminWorkspaceCandidates` boundary is **already
  covered** by `tests/forge_web_workspace_onboarding_contract.rs` (the 409
  unconfigured refusal and the candidate shape). This change adds a **tiny**
  contract test, `tests/web_workspace_status_summary_contract.rs`, that asserts
  only (a) the summary region is present in `index.html`, (b) the loader is
  wired in `app.js` with no `innerHTML` / `eval(` / `Function(` and no write
  button, and (c) a pure aggregation helper `aggregateWorkspaceSummary(candidates)`
  — extracted from the loader so it is unit-testable in Rust via a documented
  JS-equivalent — yields `registered`/`onboardable` from a fixture of candidate
  states. **The 401/403/415 boundary is not re-tested** (already covered by the
  onboarding contract test); the new test asserts only the summary-specific
  behavior and calls out that the route boundary is pre-existing.
- **Concerns:** security (the loader reuses the existing `request()` session
  gate; no new origin/path handling); quality (no placeholder, no new dep, no
  `innerHTML`, source-file-size caps); a11y (the chip group is a labelled
  `role="status"` region with `aria-live="polite"`, mirroring `source-list`);
  the **never-fabricate** invariant is preserved literally — a `409` shows
  "not configured" and no number; a `state==="registered"` sibling is the only
  thing counted as registered; an unparseable/empty payload hides the counters
  rather than showing a guessed zero.

## Capabilities

- A Forge operator signed in to the portal sees, in the topbar of "All
  projects", a read-only workspace summary: the count of registered sibling
  directories, the count of onboardable sibling directories, and a last-sync
  freshness label — aggregated live from the same candidate route the
  onboarding panel uses, with no new API surface.
- When `FORGE_ADMIN_PROJECTS_ROOT` is not configured, the operator sees an
  honest "Workspace not configured" chip instead of any count.

## Non-goals

- **No new `src/api` route or `Route` variant.** The summary aggregates from the
  existing `AdminWorkspaceCandidates` route; if a genuinely new route were ever
  needed this change would stop and escalate to the manager.
- **No write / no onboard action in the summary.** Onboarding stays on the
  existing management panel; the topbar control is a mirror only.
- **No `AdminProjects` re-aggregation for registered count.** The candidate
  route is the authoritative sibling-registration view (it already marks each
  sibling `state`); counting there avoids conflating the unrelated fleet-registry
  source with the workspace-root scan. `AdminProjects` is not read by this
  change.
- **No fabricated last-sync timestamp.** The candidate route exposes no sync
  time; the summary reports the client-side observation freshness, never a
  server timestamp it does not have.
- **No modification of `src/cli/**`, `src/api/**`, or `src/workspace/**` (Core).**
  This change is a thin browser aggregation of an existing admin read.
- **No new `src/api/router.rs` match arms.** `router.rs` stays ≤ 1000 lines.
