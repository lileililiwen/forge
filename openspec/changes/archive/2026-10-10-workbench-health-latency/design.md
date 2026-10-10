# Design: Workbench health latency

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge` crate
  (library) plus `frontend/`. No sibling touched.
- **Modules changed:**
  - `src/api/workbench.rs` — `build_health` takes the fast path;
    new `refresh_health` handler runs today's live pass.
  - `src/api/router.rs` — one new arm:
    `("POST", ["v1", "admin", "projects", id, "health", "refresh"])`
    → `Route::AdminProjectHealthRefresh { id }`.
  - `src/api/model.rs` — new `Route` variant.
  - Router exhaustiveness lists (`router.rs` 520/691/828-style arms),
    `handlers_core.rs` guards, `admin/deploy.rs` dispatch — new variant
    threaded through each, following the maintain arms exactly.
  - `src/api/command_catalog/routes.rs` (+ rows + `catalog.rs` pins) —
    one `web` row for the refresh route, referencing a new
    `ROUTE_PROJECT_HEALTH_REFRESH` constant in `workbench.rs` (same
    pattern as `maintain.rs` constants).
  - `frontend/app.js` — `HEALTH_LABELS`/`HEALTH_BADGE` gain `deferred`;
    `renderHealth` renders the `policy-deferred` row with a refresh
    button instead of a remediate shortcut; new `refreshHealth(id)`
    with progress state re-rendering the card.
  - `tests/forge_web_project_workbench_contract.rs` — state pin accepts
    `deferred`; new sentinel-oracle test (new file
    `tests/workbench_health_latency_contract.rs` to keep the change's
    oracle in one place).
  - `tests/forge_portal_frontend_contract.rs` — pin `deferred` label,
    refresh button marker, endpoint string.
- **Modules reused unchanged:** `run_doctor` (None-policy path),
  `run_driftwatch`, `DriftWatchConfig::from_env`, `RegistryObservation`,
  `resolve`, `refuse`, `guarded`, `Finding` serialization.
- **Must NOT change:** route paths except the one addition, contract
  version `forge-project-workbench/0.1.0`, journal behavior (reads
  write no rows, as today), CLI behavior, registry schema,
  `API_CONTRACT_VERSION`.

## 2. Language and runtime

- Rust 2021, `rustc 1.87` floor. Build: `cargo build`. Test:
  `cargo test --test workbench_health_latency_contract` plus the
  existing workbench/maintainer/portal suites.
- Linux; the sentinel oracle uses a shell script fixture + env override,
  no new dependency.

## 3. Ownership and shared code

- `src/api` owns the projection split: `build_health` (fast, GET) and
  `refresh_health` (live, POST) share a private `health_document`
  helper parameterized by `policy: Option<PolicyOutcome>` so the two
  paths cannot drift in shape. The deferred finding is appended by the
  fast path only.
- `frontend/` owns the card: one new button + one new state, reusing
  existing styles, live regions and the `requestStatus` helper.

## 4. User experience and interface

Actor: operator. Entry: opening any project. Fast GET returns in
milliseconds with local findings; card states:

- `issues` — local problems found (unchanged rendering).
- `stale` — observation older than manifest mtime (unchanged).
- `deferred` — local checks clean, live policy check not run for this
  view. Card shows the reason line plus **Run full health check**.
- `unavailable` — unreadable dir or failed inspection (unchanged).

The `policy-deferred` finding row renders its `detail` text with the
refresh button (never a remediate shortcut — there is nothing to
remediate). While the refresh runs, the button disables with
"Running full check…"; on success the card re-renders with the full
document (`healthy`/`issues`/`stale`, full findings); on failure an
honest error notice appears and the prior card state is kept. Narrow
viewports reuse the existing card layout; no new CSS. Contrast and
keyboard behavior reuse existing tokens and focus hooks.

## 5. Behavioral model

```
GET detail → resolve → manifest (path stripped)
           → build_health_fast:
               !available/dir → unavailable + note (unchanged)
               run_doctor(dir, None, obs, None)
                 Ok(report) → strip path → append policy-deferred
                              unavailable finding → state:
                              stale if report.stale,
                              issues if !report.healthy,
                              deferred otherwise
                 Err(_) → unavailable + note (unchanged)
           → operations + workflows (unchanged)

POST health/refresh → resolve (same refusal) → run_driftwatch +
           run_doctor(policy=Some) [today's live pass, verbatim]
           → strip path → state healthy/issues/stale (no deferred
           finding) → 200 full document. No confirm, no digest, no
           journal row (read-only effect; response is the evidence).
```

Determinism: finding order = doctor order + deferred appended last;
no timestamps in the appended finding.

## 6. Contract and compatibility

- `health.state`: `healthy|stale|issues|unavailable` + `deferred`.
  `healthy` is now only ever emitted by the refresh route (full pass
  ran). Any consumer matching the old four states exhaustively must
  handle `deferred` — compiler-checked in Rust (`HEALTH_LABELS`
  fallback `|| state` in JS keeps old frontend safe).
- Findings: new possible id `policy-deferred`
  (`status: "unavailable"`, `applicable: false`,
  `remediation: "manual"`, `detail` naming the refresh action).
- New route constant
  `ROUTE_PROJECT_HEALTH_REFRESH = "POST /v1/admin/projects/{id}/health/refresh"`,
  catalogued `web`.
- Wire/back-compat: old frontends render `deferred` via the `|| state`
  fallback; the remediate shortcut on the deferred row would offer a
  bogus plan — new frontend special-cases it (old frontends keep
  today's honestly-refused preview behavior).

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| Unreadable dir (GET or refresh) | `unavailable` + note, nothing run |
| Unknown/unmanaged id | existing 400/404 refusal, untouched |
| Refresh with no checker binary | 200 full document, policy finding `unavailable` naming the cause (today's GET behavior, now on demand) |
| Refresh checker timeout/unparseable | same, reason carried |
| Refresh on non-POST / wrong path | router 404/405 as today |
| Sentinel test env without exec bit | test sets permissions explicitly; failure is a test bug, loud |
| `HEALTH_LABELS` missing `deferred` | frontend contract test pins it |

## 8. Verification oracle

- **New `tests/workbench_health_latency_contract.rs`**: fixture
  project + sentinel driftwatch script (`FORGE_DRIFTWATCH_BIN` →
  touches `$SENTINEL`, prints minimal checker JSON): GET detail 200,
  state in {deferred, issues, stale}, sentinel untouched,
  `policy-deferred` present when state is deferred; POST refresh 200,
  sentinel touched, deferred finding absent. Env override is
  process-scoped per test (serial, no parallel interference — Rust
  test threads share env; use a mutex or run single-threaded via
  `--test-threads=1` in the test invocation, documented in the file).
- **Updated workbench contract pin**: `deferred` accepted.
- **Frontend source contract**: `deferred` label, refresh endpoint
  string, button marker.
- **Existing suites green**: workbench, maintainer surface, portal UI,
  doctor, policy unit tests.
- A task box is checked only with the command output for its assertion.

## 9. Decision ledger

- **Resolved:** fast GET + explicit refresh (HANDOFF candidate b +
  local projection) over a cache layer — no schema change, no
  invalidation semantics to get wrong, no invented passes.
- **Resolved:** new `deferred` state over reusing `stale` — `stale`
  already means observation-vs-mtime; overloading it would lie
  differently.
- **Resolved:** refresh is confirm-free and journal-free — read-only
  effect; the response is the evidence. Matches preview (no rows for
  reads) precedent.
- **Resolved:** sentinel-binary oracle over wall-clock assertions —
  timing tests are flaky; invocation proof is deterministic.
- **Deferred to owner ruling:** retire/death lifecycle (Defect 2).
- **Blockers:** none.
