# core-http-api Specification (delta)

## ADDED Requirements

### Requirement: Table-driven route dispatch

Forge SHALL classify every `Route` variant against the three
cross-cutting dispatch tables — required permission, admin
short-circuit, and admin fallthrough — in a single side module
(`src/api/router_classify.rs`) so the `src/api/router.rs` file
remains under the `source-file-size` cap and so a future variant
is registered in one place (the new module) rather than threaded
through three or more cross-cutting `match` arms in `router.rs`.

The classification module SHALL expose exactly four functions
(the first three are exhaustive `match` expressions on `&Route`,
the fourth is a thin wrapper that returns the existing `not_found()`
envelope for admin variants that did not short-circuit):

- `pub(super) fn required_permission(route: &Route) -> Option<&'static str>`
  — answers the permission the session gate requires for the
  variant, or `None` if no session is required; byte-identical
  classification to the pre-refactor `required_permission` function
  in `src/api/router.rs`.
- `pub(super) fn is_admin_dispatch(route: &Route) -> bool` —
  returns `true` for every variant that short-circuits to
  `super::admin::handle(config, db_path, request, &route)`, including
  the `Route::AdminProjectGateStatus` / `Route::AdminProjectGateRun`
  variants added for the workbench gate controls; byte-identical
  to the pre-refactor `matches!` arm in `src/api/router.rs`.
- `pub(super) fn is_admin_route(route: &Route) -> bool` — returns
  `true` for every admin-classified variant; byte-identical to the
  pre-refactor final-dispatch fallthrough in `src/api/router.rs`.
- `pub(super) fn admin_fallthrough(route: &Route) -> ApiResponse` —
  returns the existing `not_found()` envelope for an admin variant
  that did not short-circuit (an unreachable load-bearing
  exhaustiveness check); byte-identical refusal to the pre-refactor
  fallthrough.

The path table (`src/api/router.rs::route_request`) SHALL remain
path-driven and per-route (literal-segment matching with the
established "literal precedes generic `{id}`" precedence); the
refactor MUST NOT collapse the path table to a data-driven
registry because that would risk changing the dispatch for every
existing path. The per-handler dispatch in
`src/api/router.rs::handle` (the `Route::Healthz | Route::ListProjects | …`
core arms) SHALL remain per-handler because each arm names a
different handler function; the refactor MUST NOT collapse that
match to a registry.

The public API surface — `pub fn handle(config, db_path, request)`,
the `Route` enum visibility, and the `ApiRequest` / `ApiResponse`
shape — SHALL be unchanged. The wire shape for every existing
variant — the 200 / 401 / 403 / 415 / 404 / 409 envelopes, the
`admin:access` permission table, the path-bound 5-segment and
6-segment `OPTIONS` preflights, the `Route::AdminOptions`
short-circuit, and the `alt_method` 405 promotion — SHALL be
preserved byte-identically for every existing variant.

#### Scenario: Adding a new admin route touches `router_classify.rs` only

- **WHEN** a future `Route::AdminFoo` variant is added beside its
  handler in `src/api/admin/<thing>.rs` and added to the `Route`
  enum
- **THEN** the change adds zero lines to `src/api/router.rs`: the
  path arm is in the beside-handler matcher
  (`super::admin::route_beside`), the permission classification is
  one arm in `router_classify::required_permission`, the
  admin-dispatch classification is one arm in
  `router_classify::is_admin_dispatch`, and the fallthrough
  classification is one arm in `router_classify::is_admin_route`;
  the change adds at most 3 lines to `router_classify.rs` and
  zero lines to `router.rs`, so the `source-file-size` cap is
  preserved for the next 25+ variants

#### Scenario: A new variant that misses a classification fails to compile

- **WHEN** a future `Route` variant is added without a
  classification arm in one of the three exhaustive `match`
  expressions in `router_classify.rs`
- **THEN** the Rust compiler emits a non-exhaustive-match error
  pointing at the missed arm, exactly as the pre-refactor
  cross-cutting arms did — the compile-time safety net is
  preserved, the only change is that the error surface is
  centralized in one file

#### Scenario: Refactor preserves the 401 refusal for every admin route

- **WHEN** an anonymous request is issued against any admin
  variant — including `Route::AdminProjectGateStatus` and
  `Route::AdminProjectGateRun` — after the refactor
- **THEN** the response is `401 api-unauthorized` (the same
  refusal the pre-refactor cross-cutting arms produced), proving
  the `is_admin_dispatch` table is load-bearing and that the
  pre-dispatch path is preserved byte-identically

#### Scenario: Refactor preserves the permission table for every variant

- **WHEN** a session-gated request is issued against any variant
  after the refactor
- **THEN** the `required_permission` classification matches the
  pre-refactor table exactly — the `admin:access` cluster
  (CreateProject, AddFeature, UpgradeProject, GenerateSpec,
  AgentTransition, PortfolioTag, PortfolioRelation,
  PortfolioReview, PortfolioEvidence, ApplyDeployment, the share
  cluster, the interest cluster, the delivery cluster) demands
  `admin:access`, every other gated variant demands a session
  with no extra permission, and the public routes
  (`Route::Healthz`, `Route::GetOperation`, `Route::GitHubPush`)
  require no session at all

#### Scenario: Mutation test pins the classification tables

- **WHEN** the `is_admin_dispatch` table is mutated by removing
  one row (the `Route::AdminProjectGateRun` row, picked because
  the previous subagent just added it)
- **THEN** the route that previously short-circuited to
  `super::admin::handle` falls through to `admin_fallthrough`
  and returns the existing `not_found()` envelope — the same
  refusal the pre-refactor code produced when a route was not
  matched by the admin pre-dispatch — proving the
  classification tables are the load-bearing surface and that
  the refactor is structurally equivalent to the cross-cutting
  `match` arms it replaced
