# rust-identity-audit-permission Specification

## Purpose

The `rust-identity-audit-permission` capability closes the gap
between the foundation's trait surface and the application-level
helpers every consumer needs. It adds:

* a `PermissionEvaluator` trait with `DenyByDefaultEvaluator` and
  `AllowListEvaluator` reference implementations;
* a `TenantResolver` trait with a `NoTenantResolver` placeholder
  implementation;
* `AuthenticationAuditor`, `PermissionAuditor`, and `SessionAuditor`
  audit-event producers that wrap any `AuditSink` and emit canonical
  `AuditEvent` values for the most common event types.

The new surface is opt-in and application-replaceable. The
application still owns its policy, store, provider, routing, and
audit storage; the foundation now owns the wire-format and the
audit category / reason code catalog.

## ADDED Requirements

### Requirement: Permission evaluator

The identity crate MUST expose a synchronous `PermissionEvaluator`
trait that takes a resolved `Subject`, an action key, a resource
identifier, and a tenant id, and returns a `PermissionEvaluation`
with a stable `PermissionReason` and an optional policy id.

The identity crate MUST ship two reference implementations:
`DenyByDefaultEvaluator` (deny everything) and `AllowListEvaluator`
(allow only listed `(SubjectId, Permission)` entries, default deny
otherwise).

#### Scenario: Deny by default

- **WHEN** an application wires a `DenyByDefaultEvaluator`
- **THEN** every evaluation returns `PermissionEvaluation::Deny`
  with `PermissionReason::Other` and a `None` policy id, regardless
  of subject, action, or resource

#### Scenario: Allow list matches and misses

- **WHEN** the `AllowListEvaluator` is queried for an allowed
  `(SubjectId, Permission)` pair
- **THEN** it returns `PermissionEvaluation::Allow` with
  `PermissionReason::RoleMatch`
- **AND WHEN** it is queried for any other pair
- **THEN** it returns `PermissionEvaluation::Deny` with
  `PermissionReason::MissingRole`

### Requirement: Tenant resolver

The identity crate MUST expose a synchronous `TenantResolver` trait
that takes a resolved `Subject`, a `Clock` reference, and a
`CorrelationId`, and returns an `IdentityOutcome<TenantContext>`.

The identity crate MUST ship a `NoTenantResolver` reference
implementation that returns
`IdentityOutcome::Failed(IdentityError::Misconfigured)` for every
input, so applications can wire the trait without committing to a
particular store.

#### Scenario: Misconfigured resolver

- **WHEN** a `NoTenantResolver` is asked to resolve any subject
- **THEN** it returns
  `IdentityOutcome::Failed(IdentityError::Misconfigured)` and the
  error never includes a credential or provider response detail

### Requirement: Audit event producers

The audit crate MUST expose `AuthenticationAuditor`,
`PermissionAuditor`, and `SessionAuditor`, each wrapping an
`AuditSink` and a `Clock`, with `record_*` methods that emit
canonical `AuditEvent` values for the corresponding event types.

Every producer MUST reject context keys that match a registered
secret substring at construction time and MUST NOT embed the
source error message into the audit event payload.

#### Scenario: Permission denial event

- **WHEN** a `PermissionAuditor::record_evaluation` is called with
  a `PermissionEvaluation::Deny`
- **THEN** the recorded event has `category = Permission`,
  `outcome = Denied`, and an empty or non-secret `details` map
- **AND** the event never includes the source subject's password,
  token, hash, or provider response detail

#### Scenario: Session revocation event

- **WHEN** a `SessionAuditor::record_session_revoked` is called
- **THEN** the recorded event has `category = Lifecycle`,
  `outcome = Denied`, and the session id is recorded via the
  resource field, not the details map

### Requirement: Test-only scripted evaluator

The test-only `platform-testing` crate MUST expose a
`ScriptedPermissionEvaluator` that returns the next pre-recorded
`PermissionEvaluation` in FIFO order and falls back to a
configurable default evaluator (defaults to
`DenyByDefaultEvaluator`) once the script is exhausted.

#### Scenario: Scripted evaluator determinism

- **WHEN** a test wires a `ScriptedPermissionEvaluator` with a
  pre-recorded sequence of allow and deny outcomes
- **THEN** successive calls return the pre-recorded outcomes in
  order, regardless of the subject, action, or resource
- **AND** calls after the script is exhausted return the fallback
  outcome
