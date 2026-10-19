# Proposal: Identity, audit, and permission contracts

## Why

The foundation change established the crate boundaries, language-neutral
contract envelopes, and provider-neutral trait surface. It deliberately
shipped **types and traits only**; every consumer would still have to
hand-roll the application-level helpers that thread identity, permission,
tenant, and audit events together. Without those helpers, every consumer
reinvents:

* a permission evaluator and the wire format of its denial reasons;
* a tenant resolver that maps a resolved `Subject` to a `TenantContext`;
* audit event emission for authentication, permission, and session
  lifecycle events, with the category, outcome, and redaction policy
  re-derivation in every consumer.

That drift is the actual cost of the trait-only foundation. The
`identity-audit-and-permission-contracts` change closes the gap with
small, opt-in, application-replaceable helpers built on top of the
foundation. The application still owns the policy, the store, the
provider, the routing, and the storage of audit events; the foundation
now owns the wire-format and the audit category / reason code catalog.

## What changes

* `platform-identity` gains a `PermissionEvaluator` trait and two
  reference implementations (`DenyByDefaultEvaluator`,
  `AllowListEvaluator`) built on the existing `Permission`,
  `PermissionDecision`, and `PermissionReason` types.
* `platform-identity` gains a `TenantResolver` trait and a
  `NoTenantResolver` reference implementation that returns
  `IdentityError::Misconfigured` for every input.
* `platform-audit` gains three thin audit-event producer helpers
  (`AuthenticationAuditor`, `PermissionAuditor`, `SessionAuditor`)
  that wrap any `AuditSink` and build canonical `AuditEvent` values
  for the most common event types. Each producer re-checks the
  redaction policy at emit time and never embeds the source error
  message into the audit payload.
* `platform-testing` gains a `ScriptedPermissionEvaluator` for tests
  that need deterministic allow/deny sequences.

## BFS impact map

* Capabilities: typed evaluators, audit producers, tenant resolution.
* Consumers: same set as the foundation
  (`openaccounting`, `rust-ecommerce`, `hookit`, `openpanel`,
  `vowra`, future Rust services). The change is opt-in; consumers
  that already have their own helpers can ignore the new surface.
* Persistence: none. Evaluators, resolvers, and sinks are
  application-supplied.
* Security: no credential material ever crosses the new boundary;
  every producer re-checks against the `RedactedContext` policy.
* Compatibility: contract identifiers unchanged. The change adds new
  Rust types and traits; it does not modify any contract envelope
  or wire format.
* Tests: per-trait unit tests, an end-to-end
  identity -> permission -> audit integration test, and a regression
  check that the foundation's 48-case cross-repository fixture
  matrix still passes.
* Architecture: the architecture test is extended to forbid the new
  surface from pulling in `axum`, `sqlx`, `cookie`, `jsonwebtoken`,
  `openidconnect`, `oauth2`, `reqwest`, or `tokio` (direct or
  transitive) and to forbid production crates from depending on
  `platform-testing` (direct or transitive).

## Capabilities

### `rust-identity-audit-permission`

Applications can evaluate permissions, resolve tenant context, and
emit canonical audit events using only platform types and traits.
The new surface is small, opt-in, and application-replaceable; the
application's existing helpers continue to work unchanged.

## Non-goals

* No RBAC policy DSL, no role inheritance, no policy file format.
* No OIDC / JWT validation, no cookie handling, no HTTP, no database.
* No session storage, no audit storage, no migration tooling.
* No migration of `openaccounting`, `rust-ecommerce`, `hookit`,
  `openpanel`, or `vowra`; adoption remains a per-consumer
  follow-up change.
* No change to the `platform-contracts` JSON Schemas, registry, or
  cross-repository fixture matrix.
