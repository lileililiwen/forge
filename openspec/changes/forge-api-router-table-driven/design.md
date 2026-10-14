# Design: forge-api-router-table-driven

## Ownership

The three cross-cutting `match` arms in `src/api/router.rs` are
replaced by per-variant methods on `&Route`, defined in a new side
module `src/api/router_classify.rs` (declared as
`pub(super) mod router_classify;` in `src/api/mod.rs`). The methods
are exhaustive `match` expressions that answer the same three
questions the cross-cutting arms answer today, and they are the
single place a future variant is classified against the three
dispatch tables. The methods are `pub(super) fn` so `router.rs` is
the only caller, and they live in a separate file because the
classification tables are the change-shaped surface and keeping them
beside `router.rs` (instead of in `model.rs`) preserves the
"dispatch lives in `router.rs`" boundary.

`src/api/router.rs` itself becomes the dispatch file: it owns the
path table (`route_request`), the `handle` function, the
`method_not_allowed` / `not_found` / `bad_request` helpers, and the
`alt_method` 405 promotion. The cross-cutting `match` arms become
`if super::router_classify::is_admin_dispatch(&route) { return ... }`
and a final `route => super::router_classify::admin_fallthrough(route)`
arm. The path table and the per-handler dispatch in `handle()` are
unchanged because they are path-driven and handler-driven, not
classification-driven.

The `Route` enum (`src/api/model.rs`) is unchanged. No new variant,
no new visibility, no new field. The methods on `&Route` are
defined in `router_classify.rs` so the enum file does not grow.

The "beside the handlers" pattern in `src/api/admin/{agent_identity,
history, creation, shipping_reads, assurance, identity, gate}.rs` is
unchanged. `super::admin::route_beside(method, segments)` runs
before `route_request` (line 100) and catches every beside-handler
route; the new `router_classify.rs` is consulted only after the
path table returns a `Route`.

## Contracts

### Module shape

```
src/api/router_classify.rs    (new, ≤ 200 lines)
  pub(super) fn required_permission(route: &Route) -> Option<&'static str>
  pub(super) fn is_admin_dispatch(route: &Route) -> bool
  pub(super) fn is_admin_route(route: &Route) -> bool
  pub(super) fn admin_fallthrough(route: &Route) -> ApiResponse
```

### `required_permission(route: &Route) -> Option<&'static str>`

Returns the permission string the session gate requires for the
route, or `None` if no session is required. The exhaustive `match`
preserves the exact same classification as the existing
`required_permission` function (`src/api/router.rs:566-706`):
- `Route::Healthz | Route::GetOperation { .. } | Route::GitHubPush`
  → `None`.
- Every `Route::Admin*` variant → `None` (admin session is enforced
  separately by the admin dispatch, which runs before
  `required_permission`).
- The `admin:access` cluster (`Route::CreateProject | AddFeature |
  UpgradeProject | GenerateSpec | AgentTransition | PortfolioTag |
  PortfolioRelation | PortfolioReview | PortfolioEvidence |
  ApplyDeployment | GetShare | SetShare | RemoveShare | SharePreview |
  ShareApprove | SharePublish | ShareReconcile | ShareAudit |
  GetInterest | ImportInterest | InterestCompare | InterestTrend |
  InterestAudit | InterestReadiness | DeliveryStatus | DeliveryPreflight
  | DeliveryStage | DeliveryPromote | DeliveryHermoraRetry`)
  → `Some("admin:access")`.
- Every other variant → `None`.

### `is_admin_dispatch(route: &Route) -> bool`

Returns `true` for every variant that short-circuits to
`super::admin::handle(config, db_path, request, &route)`. The
exhaustive `match` lists every `Route::Admin*` variant that is
**not** `Route::AdminOptions` (the OPTIONS variant short-circuits
to `super::admin::handle_preflight` and is handled separately).
The list is byte-identical to the existing `matches!` arm in
`src/api/router.rs:758-841`, including the `Route::AdminProjectGateStatus`
/ `Route::AdminProjectGateRun` rows added by the previous subagent.

### `is_admin_route(route: &Route) -> bool`

Returns `true` for every admin-classified variant (the union of
`is_admin_dispatch` plus `Route::AdminOptions` plus the
`Route::AdminSession*` / `Route::AdminProjects` / `Route::AdminCommands`
variants that the existing final-dispatch fallthrough treats as
`not_found()`). The list is byte-identical to the existing
fallthrough arm in `src/api/router.rs:924-996`.

### `admin_fallthrough(route: &Route) -> ApiResponse`

For an admin route that did not short-circuit (which is unreachable
in practice because `is_admin_dispatch` covers every admin variant
that has a real handler), the function returns `not_found()`. The
function exists so the final-dispatch match in `handle()` collapses
to a single `route => super::router_classify::admin_fallthrough(route)`
arm — the unreachable arm is a load-bearing exhaustiveness check.

### `router.rs` shape (after)

- Line count drops from 1006 to ~915.
- The 138-line `required_permission` function is deleted; the
  caller in `handle()` becomes
  `super::router_classify::required_permission(&route)` (or
  `authorize(config, db_path, &route, request, now)` is unchanged —
  the authorize function calls `required_permission` internally).
- The 86-line `matches!` arm at line 758-841 collapses to
  `if super::router_classify::is_admin_dispatch(&route) { return
  super::admin::handle(config, db_path, request, &route); }`.
- The 75-line fallthrough arm at line 924-996 collapses to
  `route => super::router_classify::admin_fallthrough(route)`.
- The path table (`route_request`) is unchanged.
- The per-handler dispatch match in `handle()` (the
  `Route::Healthz | Route::ListProjects | …` core arms) is
  unchanged — only the trailing admin fallthrough is collapsed.

## Failures

Every existing 401 / 403 / 415 / 404 / 409 envelope is preserved.
The refactor is classification-only; the response shapes are
unchanged. The failure modes are:

1. **A future `Route` variant is added without classification.**
   Adding a new variant to `Route` triggers a compile error in
   `router_classify.rs` (the three exhaustive `match` expressions
   are exhaustive), exactly like the existing cross-cutting `match`
   arms. The compile error is the same shape: "missing match arm".
   The refactor preserves the compile-time safety net; the new
   `router_classify.rs` is the single place to add the arm.
2. **A future `Route` variant is added to one classification but
   not another.** The exhaustive `match` ensures this is a compile
   error in the missed arm. The cross-cutting arms today give the
   same compile error; the refactor centralizes the
   compile-error surface in `router_classify.rs`.
3. **A `required_permission` arm is wrong.** The mutation test in
   `tests/router_table_driven_contract.rs` exercises
   `forge::api::handle` against a fresh fixture for every existing
   `Route` variant and asserts the response status matches the
   pre-refactor behavior. A wrong arm produces a different status
   for that variant and the test fails.
4. **An `is_admin_dispatch` arm is wrong.** The mutation test
   removes the `Route::AdminProjectGateRun` row from
   `is_admin_dispatch`, re-runs the dispatch, and asserts the
   `gate/run` route that previously short-circuited to
   `super::admin::handle` now falls through to
   `admin_fallthrough` → `not_found()`. This is the
   "the table is load-bearing" test.
5. **An `is_admin_route` arm is wrong.** Same mutation test shape
   as `is_admin_dispatch`: the test pins a known admin variant
   and asserts the fallthrough refusal reproduces.
6. **`router_classify::admin_fallthrough` is wrong for an admin
   route that did not short-circuit.** The function returns
   `not_found()`; the only way it is wrong is if it returns
   anything other than `not_found()`. The mutation test pins the
   `not_found()` envelope.

## Migrations

None. The refactor is local to `src/api/`:

- No new `Route` variant — the enum is unchanged.
- No new public API — `forge::api::handle`,
  `forge::api::route_request`, the `Route` enum visibility, and
  the `ApiRequest` / `ApiResponse` shape are unchanged.
- No new dependency — only `Route`, `super::admin`, and the
  existing imports of `router.rs` are touched.
- No new external contact — no provider, no adapter, no network,
  no journal row, no write, no shell.
- No schema change — the `Route` enum layout is unchanged.
- No new CLI verb — the CLI is the source of truth and is
  unchanged.
- No new file under `tests/` is required by the existing test
  suite; only the new mutation contract binary
  `tests/router_table_driven_contract.rs` is added.

Source-file-size posture:
- `src/api/router.rs` shrinks from 1006 → ~915 lines (cap
  respected, comfortable headroom).
- `src/api/router_classify.rs` (new) ≤ 200 lines (cap respected,
  well below).
- `src/api/model.rs` — **unchanged**.
- `src/api/admin/*` — **unchanged** (the gate / agent_identity /
  history / creation / shipping_reads / assurance / identity
  beside-handler matchers keep working because `route_beside`
  runs before `router_classify` is consulted).
- `src/cli/**` — **unchanged**.
- `src/api/command_catalog/**` — **unchanged**.

The refactor is an internal-routing change only: the public API
surface is preserved, the wire shapes are preserved, the failure
envelopes are preserved, and the per-handler dispatch is preserved.
The only thing that changes is **where** the cross-cutting
classification lives — from three exhaustive `match` arms in
`router.rs` to three exhaustive `match` arms in `router_classify.rs`
called by `router.rs`.
