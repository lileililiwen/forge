# Tenancy sample (stage 4)

Tenant lifecycle orchestration with application-owned workflow,
steps, scope, and store.

## Owned by the application

- `SampleProvisioningWorkflow` and its steps, including step order,
  idempotency, and tenant scoping.
- `SampleTenantLifecycleStore`: the durable operation record
  (in-memory here; production code persists it in the application's
  own database).
- The tenant identifier and the decision of when to start or resume
  an operation.

## Owned by the platform

- The orchestration contract: ordered execution, resume skipping
  completed steps, failure classification, and safe status snapshots.
- The development `InMemoryTenantLifecycleStore` and no-op scope
  defaults, both replaced above by application registrations.

## Rollback

Remove the `Platform.Tenant.Lifecycle*` references and run the two
steps inline; the workflow and store classes keep working without
the orchestrator.

## Non-goals

No multitenant query filters, tenant resolution middleware, or
per-tenant connection routing. Those belong to the application's
data-access layer.
