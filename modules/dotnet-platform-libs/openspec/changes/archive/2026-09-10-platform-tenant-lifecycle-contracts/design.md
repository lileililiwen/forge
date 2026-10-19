## Context

Starter paths `src/Modules/Multitenancy/Modules.Multitenancy/Provisioning` and `Services/TenantExpiryScanJob.cs` show repeated lifecycle behavior. Platform multitenancy intentionally owns no tenant catalog. The design must preserve that boundary while making provisioning status and retry behavior portable.

## Goals / Non-Goals

**Goals:**

- Define a deterministic lifecycle state machine and step contract.
- Support idempotent retries and operator-triggered retry without duplicate side effects.
- Integrate with jobs and readiness without requiring a scheduler or database.

**Non-Goals:**

- Own tenant storage, Finbuckle configuration, connection-string secrets, migrations, or seed data.
- Decide tenant expiry, billing, suspension, or deletion policy.
- Build a tenant admin UI.

## Decisions

- Use opaque `TenantId` values and application-provided `ITenantLifecycleStore`/catalog seams.
- Model each step with a stable name, order, idempotency key, timeout/cancellation, and retry classification.
- Persist state through an application adapter; provide only an in-memory implementation for tests.
- Expose status as a provider-neutral snapshot consumed by readiness or admin adapters.
- Integrate jobs through `IJobDispatcher` rather than referencing Hangfire.

## Risks / Trade-offs

- [Risk] A step is not actually idempotent → require step idempotency documentation and retry tests; do not claim transactional orchestration across external systems.
- [Risk] Tenant context leaks between steps → require explicit scope creation/restoration per step.
- [Risk] Migration execution can be destructive → expose an application callback and status, never call EF migration APIs from the core contract.

## Migration Plan

1. Add contracts and an in-memory state store.
2. Pilot provisioning for one application-owned tenant workflow.
3. Add optional job and ASP.NET Core status adapters.
4. Migrate individual steps while retaining the application's current provisioning path as rollback.
