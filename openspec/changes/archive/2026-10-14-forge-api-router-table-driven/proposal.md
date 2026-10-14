# Proposal: forge-api-router-table-driven

## Why

`src/api/router.rs` is at **1006 / 1000** lines (HANDOFF.md evidence
table, `source-file-size` row, 2026-10-14 entry). The over-shoot is
forced by Rust's exhaustive `match`: every new `Route` variant must be
listed in every cross-cutting `match` arm that covers it. The recent
`Route::AdminProjectGateStatus` / `Route::AdminProjectGateRun` addition
(prior subagent's `forge-web-gate-controls`) added **6 lines** in
`router.rs` — one match arm in the path table, two arms in the
permission table, two arms in the admin pre-dispatch, one arm in the
final dispatch. The existing pattern of "add 1–2 lines per new variant
in `router.rs`" is now structurally unreachable: the cap is exceeded
*before* the next variant is added.

The "kept beside the handlers" pattern (`agent_identity.rs`,
`history.rs`, `creation.rs`, `shipping_reads.rs`, `assurance.rs`,
`identity.rs`, `gate.rs`) has already moved ~30 read-only / beside-handler
routes out of `router.rs` — but the four *cross-cutting* `match` arms
still grow by 1–2 lines per variant because every variant must be
classified against the four dispatch tables. The path table
(`route_request`) is path-driven and must remain literal-segment
matching; the other three are pure `Route`-variant classification and
can be **per-variant methods** on the dispatch.

`source-file-size` is the gate check that fails today; the cap is
recoverable now, but the next sibling-empowerment subagent that adds a
new admin route will exceed it again. Refactor so the next variant is
zero lines in `router.rs`: a new admin route is registered beside its
handler (in `admin/<thing>.rs`), added to the `Route` enum, and the
three cross-cutting `match` arms collapse to per-variant methods that
read the variant's own classification.

## What Changes

- **Three per-variant methods on `Route`** (in a new side module
  `src/api/router_classify.rs`, beside `router.rs`, so the enum file
  `model.rs` is not touched). The methods are exhaustive `match`
  expressions on `&Route` and answer three questions:
  - `pub(super) fn required_permission(route: &Route) -> Option<&'static str>`
    — replaces the 138-line `required_permission` match in
    `router.rs:566-706` with a single function call from the same
    module.
  - `pub(super) fn is_admin_dispatch(route: &Route) -> bool`
    — replaces the 86-line `matches!` arm in
    `router.rs:758-841` with a single `if route.is_admin_dispatch()`
    guard. Returns `true` for every variant that short-circuits to
    `super::admin::handle`.
  - `pub(super) fn is_admin_route(route: &Route) -> bool`
    — replaces the 75-line final-dispatch `not_found()` fallthrough
    in `router.rs:924-996` with a single guard. Returns `true` for
    every admin variant; the fallthrough returns `not_found()` when
    the pre-dispatch did not claim the route.
- **`src/api/router.rs` shrinks from 1006 → ~915 lines**:
  - `required_permission` (line 566-706, 138 lines) deleted; replaced
    by `super::router_classify::required_permission(route)`.
  - The 86-line admin pre-dispatch `matches!` arm (line 758-841)
    collapses to `if super::router_classify::is_admin_dispatch(&route) {`
    `return super::admin::handle(config, db_path, request, &route); }`.
  - The 75-line final-dispatch fallthrough (line 924-996) collapses
    to `route => super::router_classify::admin_fallthrough(route, not_found)`.
  - The path table (`route_request`, line 103-522) is **unchanged**:
    it is path-driven and per-route; collapsing it is out of scope.
  - The handler-dispatch match (line 854-916, the
    `Route::Healthz | Route::ListProjects | …` core arms) is
    **unchanged** because it is the actual per-handler dispatch
    (each arm names a different handler function). The only change
    in that match is replacing the trailing admin fallthrough.
- **Zero new `Route` variants, zero new routes, zero new tests of
  behavior** beyond the mutation contract. The `forge-web-gate-controls`
  routes (and every other existing route) keep working with the new
  dispatch — verified by mutation tests in
  `tests/router_table_driven_contract.rs`.
- **Zero new dependencies.** Only `Route`, `super::admin`, and the
  existing imports of `router.rs` are touched.
- **Zero public API change.** `pub fn handle(config, db_path, request)`,
  the `Route` enum visibility, and `ApiRequest` / `ApiResponse` are
  unchanged.
- **Spec delta in `core-http-api`** (the canonical HTTP API spec).
  A new "Table-driven route dispatch" requirement is ADDED — it
  captures the per-variant method contract and the invariant that the
  path table remains path-driven while the cross-cutting arms become
  per-variant. The existing `core-http-api` requirements ("Equivalent
  HTTP domain behavior", "Authorized asynchronous operations") are
  unchanged: the refactor preserves both, including the 401/403/415
  refusal paths and the `admin:access` permission table for every
  existing variant.

## BFS Impact Map

- **Requirements/scenarios:** `core-http-api` (preserved — the
  refactor preserves every existing 200 / 401 / 403 / 415 / 404
  response for every existing variant). New "Table-driven route
  dispatch" requirement added to `core-http-api`.
- **Concepts/modules:**
  - `src/api/router.rs` — three cross-cutting `match` arms collapse
    to per-variant method calls (line count drops by ~90 lines).
  - `src/api/router_classify.rs` (new) — three exhaustive `match`
    expressions on `&Route` answering the three questions above. The
    file sits beside `router.rs` so the dispatch tables and the
    classifier live next to each other; total ≤ 200 lines.
  - `src/api/mod.rs` — one new `pub(super) mod router_classify;` line.
  - `src/api/model.rs` — **unchanged** (no new variants, no new
    visibility).
  - `src/api/admin/*` — **unchanged** (the gate / agent_identity /
    history / creation / shipping_reads / assurance / identity
    beside-handler matchers keep working because `route_beside`
    runs before `router_classify` is consulted).
- **Contracts:** the wire shape is byte-identical for every existing
  variant. The 401 / 403 / 415 / 200 / 404 / 409 envelopes, the
  `admin:access` permission table, the path-bound 5-segment and
  6-segment `OPTIONS` preflights, the `Route::AdminOptions` short
  circuit, and the `alt_method` 405 promotion are preserved. No
  contract version change.
- **Callers/persistence:** no Core / CLI / registry-schema change.
  No new caller; the dispatch surface (`forge::api::handle`,
  `forge::api::route_request`) is unchanged.
- **Integrations:** no provider, no adapter, no network, no new
  contact. The refactor is local to `src/api/`.
- **Tests/compatibility:** new contract binary
  `tests/router_table_driven_contract.rs`. The new test:
  - **Static-table coverage:** for every `Route` variant (covering
    all of `Route::Healthz`, the bearer `/v1` core, the `/v1` admin
    pre-dispatch, and the final-dispatch fallthrough), constructs a
    minimal `ApiRequest` whose path/method would map to that
    variant in the **pre-refactor** `route_request` table, calls
    `forge::api::handle` against a fresh fixture, and asserts the
    response status matches the pre-refactor behavior for that
    variant (a typed `401` for admin routes without a session, a
    typed `404` for an unknown project id, a typed `200` for
    `/healthz`). The exhaustive coverage proves the per-variant
    classification is identical to the cross-cutting `match` arms.
  - **Mutation test:** removes one entry from the new
    `is_admin_dispatch` table (the `Route::AdminProjectGateRun`
    row, picked because the previous subagent just added it),
    re-runs the dispatch, and asserts the route that previously
    short-circuited to `super::admin::handle` now returns
    `not_found()` — the exact pre-refactor refusal. This is the
    "if the side table is wrong, the route falls through" test
    that proves the refactor's classification is load-bearing.
  - The 401 / 403 / 415 boundary tests are not duplicated because
    the existing `tests/web_shipping_provider_reads_contract.rs`
    and `tests/web_assurance_browser_contract.rs` already cover
    that boundary for the same path family.
- **Concerns:** security (the `admin:access` permission table is
  preserved byte-identically; mutation test on a row proves the
  table is load-bearing), quality (no placeholder, no new dep, no
  public API change, source-file-size cap satisfied at ~915 lines
  after refactor).

## Capabilities

- A future admin route registered beside its handler
  (`admin/<thing>.rs`) and added to the `Route` enum requires
  **zero** changes to `src/api/router.rs`: the path arm is in the
  beside-handler matcher, the permission classification is a single
  arm in `router_classify::required_permission`, the admin-dispatch
  classification is a single arm in
  `router_classify::is_admin_dispatch`, and the fallthrough
  classification is a single arm in `router_classify::is_admin_route`.
  The next variant costs 3 lines (one per arm) in the new
  classification module, not 4+ lines in `router.rs`.
- The `source-file-size` cap is satisfied at ~915 lines for the
  current `Route` set; the next variant is +3 lines in
  `router_classify.rs` (not `router.rs`), so the cap holds for at
  least 25 more variants before the next refactor.

## Non-goals

- Collapsing the path table (`route_request`) to a registry of
  literal patterns. The path table is path-driven and per-route;
  replacing it with a data-driven registry would shadow the existing
  "literal precedes generic `{id}`" precedence rules and would risk
  changing the dispatch for every existing path. The cap is
  recoverable without touching it.
- Collapsing the final-dispatch core handler match
  (`Route::Healthz | Route::ListProjects | …` in `handle()`). That
  match names a different handler function per arm; collapsing it
  requires a registry that maps each variant to its handler, which
  is a different refactor and out of scope.
- Renaming, deleting, or splitting any `Route` variant. The
  refactor preserves every existing variant and every existing path.
- Replacing the `Route` enum with a different type, or moving
  the admin dispatch from `super::admin::handle` to a per-module
  handler. The admin short-circuit is preserved.
- Promoting the change to a generic "Forge extension" pattern. The
  refactor is local to `src/api/router.rs` and `src/api/router_classify.rs`.
- Touching `src/cli/**`, `src/api/command_catalog/**`, or
  `src/api/admin/gate.rs` (the new gate routes from the previous
  subagent must keep working with the new dispatch — verified by
  the mutation test).
