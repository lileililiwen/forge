# Design: Identity, audit, and permission contracts

## Crate-level shape

The change adds surface to three of the four production foundation
crates plus the test-only support crate. No new crates, no new
external dependencies, no new contract identifiers.

```
crates/platform-identity
  src/
    evaluator.rs   <-- new
    tenant_resolver.rs  <-- new
    lib.rs         <-- re-exports the new public surface

crates/platform-audit
  src/
    producers.rs   <-- new
    lib.rs         <-- re-exports the new public surface

crates/platform-testing
  src/
    evaluator.rs   <-- new
    lib.rs         <-- re-exports the new public surface
```

The architecture test (`crates/platform-arch-tests/src/lib.rs`) gains
one new entry: the new surface MUST live in the production crates
listed above and MUST NOT pull in any forbidden runtime dependency
(see `proposal.md` for the list).

## Permission evaluator

```rust
pub trait PermissionEvaluator: Send + Sync {
    fn evaluate(
        &self,
        subject: &Subject,
        action: &Permission,
        resource: &str,
        tenant: &TenantId,
    ) -> PermissionEvaluation;
}

pub enum PermissionEvaluation {
    Allow { reason: PermissionReason, policy_id: Option<String> },
    Deny  { reason: PermissionReason, policy_id: Option<String> },
}
```

* **Sync trait.** Matches the foundation's choice; async adapters
  wrap calls in the runtime of their choice.
* **Result type.** A discriminated union (not a `Result`) because
  evaluation never fails in the `Result` sense — the policy either
  matched or did not. The reason code is mandatory; the policy id is
  optional (some consumers do not have policy ids).
* **Reference impls.** `DenyByDefaultEvaluator` always returns
  `Deny { reason: Other, policy_id: None }`. `AllowListEvaluator`
  holds a `HashMap<(SubjectId, Permission), ()>` and a default-deny
  fallback; an allow entry returns `Allow { reason: RoleMatch, ... }`
  and a miss returns `Deny { reason: MissingRole, ... }`.
* **Why no RBAC DSL.** The proposal explicitly defers a policy DSL
  to a follow-up change. `AllowListEvaluator` is the smallest useful
  surface that proves the trait shape.

## Tenant resolver

```rust
pub trait TenantResolver: Send + Sync {
    fn resolve(
        &self,
        subject: &Subject,
        clock: &dyn Clock,
        correlation_id: CorrelationId,
    ) -> IdentityOutcome<TenantContext>;
}
```

* **Sync trait.** Same reasoning as the permission evaluator.
* **`clock` parameter.** The resolved `TenantContext` carries an
  `active_at` timestamp; the resolver picks the clock that matches
  the application's "now" policy.
* **`NoTenantResolver`.** Returns
  `IdentityOutcome::Failed(IdentityError::Misconfigured)`. It is
  useful as a placeholder during wiring and as an explicit
  "unresolved" signal in tests.
* **Application-owned.** The application implements this against
  its tenant store; the platform does not ship a real resolver.

## Audit producers

```rust
pub struct AuthenticationAuditor<'a> { sink: &'a dyn AuditSink, clock: &'a dyn Clock }
pub struct PermissionAuditor<'a>     { sink: &'a dyn AuditSink, clock: &'a dyn Clock }
pub struct SessionAuditor<'a>        { sink: &'a dyn AuditSink, clock: &'a dyn Clock }
```

Each producer holds an `AuditSink` reference and a `Clock` reference
and exposes a small set of `record_*` methods. The methods take only
typed platform inputs; they never take a `&str` description that the
caller could fill with a secret, and they re-validate the
`RedactedContext` policy at construction time.

| Method | Audit category | Outcome |
| --- | --- | --- |
| `AuthenticationAuditor::record_success(subject, method)` | `Authentication` | `Success` |
| `AuthenticationAuditor::record_failure(failure, subject_type, tenant_id)` | `Authentication` | `Failure` |
| `PermissionAuditor::record_evaluation(subject, action, resource, evaluation)` | `Permission` | `Allow` / `Denied` |
| `SessionAuditor::record_session_created(session)` | `Lifecycle` | `Success` |
| `SessionAuditor::record_session_revoked(session, reason)` | `Lifecycle` | `Denied` (revocation is a denial-of-future-use) |

* **`IdentityError` is intentionally NOT included in
  `record_failure`.** The category is enough; the message would
  either be a safe fixed string or it would risk leaking a
  credential. Consumers that need more detail record it in their
  own application log, not in the platform audit event.
* **Redaction.** Every producer uses `RedactedContext` for the
  optional `details` field. Secret-named keys are rejected at
  construction time.
* **Correlation id.** Every produced event attaches the
  correlation id supplied by the caller.

## Test-only `ScriptedPermissionEvaluator`

`platform-testing::evaluator::ScriptedPermissionEvaluator` is a
deterministic evaluator that returns the next pre-recorded
`PermissionEvaluation` in FIFO order, or a configured fallback if
the queue is exhausted. It is `Send + Sync` so it can live behind
an `Arc` in parallel tests.

The fallback is itself a `PermissionEvaluator` (defaults to
`DenyByDefaultEvaluator`), so the test can hand out the
`ScriptedPermissionEvaluator` to the system under test and the
behaviour is still predictable after the script runs out.

## Verification

* `cargo fmt --all -- --check` — clean.
* `cargo test --workspace` — existing 69 foundation tests plus
  the new per-trait and integration tests, all passing.
* `cargo clippy --workspace --all-targets -- -D warnings` — clean.
* `node scripts/check-openspec-change-names.mjs` — ok.
* `openspec validate --changes --strict --no-interactive` —
  1/1 passed.
* `openspec validate --specs --strict --no-interactive` — after
  archive, `spec/rust-identity-audit-permission` passes alongside
  the foundation spec.
* Cross-repository fixture parity (foundation regression) — all
  48 cases in 4 families still pass.
