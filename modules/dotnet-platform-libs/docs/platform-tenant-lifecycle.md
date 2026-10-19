# Platform.Tenant.Lifecycle

Provider-neutral tenant provisioning, migration, and lifecycle orchestration.
The platform owns the state machine, the resume protocol, the safe-failure
taxonomy, and the readiness mapping. The application owns the tenant catalog,
migrations, connection strings, plans, and seed data.

## Packages

| Package | Purpose |
| --- | --- |
| `Platform.Tenant.Lifecycle.Contracts` | Framework-neutral lifecycle contracts. Zero third-party packages. |
| `Platform.Tenant.Lifecycle` | Default orchestrator and in-memory store. Depends on `Platform.Tenant.Lifecycle.Contracts` and `Platform.Core`. |
| `Platform.Tenant.Lifecycle.AspNetCore` | Readiness check, status, and resume endpoints. Depends on the lifecycle packages and the `Microsoft.AspNetCore.App` framework reference. |
| `Platform.Tenant.Lifecycle.Testing` | Deterministic fakes: `InMemoryTenantLifecycleStore`, `ScriptedLifecycleStep`, `DelegateLifecycleStep`, `StaticLifecycleWorkflow`, `RecordingLifecycleScopeCallback`. |

## State machine

`TenantLifecycleOperationState` enumerates `Pending`, `Running`, `Succeeded`,
`Retryable`, `PermanentlyFailed`, `Canceled`, and `PolicyDenied`. Steps classify
their outcomes as `Succeeded`, `Retryable`, `Permanent`, `Canceled`, or
`PolicyDenied`. The orchestrator maps a step outcome to the operation state and
stops the run on the first non-`Succeeded` outcome.

## Step contract

`ITenantLifecycleStep` requires a stable `Name`, an `Order`, an
`IsTenantScoped` flag, and an idempotent `ExecuteAsync` that returns a
`TenantLifecycleStepResult`. The platform never inspects the step body; the
application owns the side effects (migrations, seed data, plan upgrades).

`ITenantLifecycleWorkflow` is an ordered, named list of steps. The workflow
name plus the operation id is the durable identity for resume.

## Store

`ITenantLifecycleStore` is application-owned. The platform provides only the
contract plus an in-memory implementation for development and test purposes.
Production code MUST register a durable adapter (EF Core, document, or otherwise
persisted) — the in-memory store loses state on restart and is not safe for
multi-instance orchestrators.

## Scope isolation

`ITenantLifecycleScopeCallback.BeginTenantScope(tenantId)` is the application
hook for installing the tenant scope around a step. The platform never imports
`Platform.Persistence.Multitenancy`; the application binds the callback to
`ITenantScopeFactory` (or its own equivalent) at composition time. The
orchestrator installs and disposes the scope for every `IsTenantScoped = true`
step, and the scope is restored even when a step throws or the operation is
canceled.

## Resume

`ITenantLifecycleOrchestrator.ResumeAsync(operationId)` reads the recorded
status, skips `Succeeded` steps, and continues from the first non-terminal
outcome. Operators can drive resume through the bundled
`MapPlatformTenantLifecycleResume` endpoint or the orchestrator directly. The
workflow registry is the seam that lets the orchestrator resolve the workflow
by name at resume time.

## Adoption

```csharp
// 1. Register the application-owned store and scope callback.
services.AddSingleton<ITenantLifecycleStore, MyEfTenantLifecycleStore>();
services.AddSingleton<ITenantLifecycleScopeCallback>(sp =>
    new TenantScopeCallback(sp.GetRequiredService<ITenantScopeFactory>()));

// 2. Register the orchestrator and workflow registry.
services.AddPlatformTenantLifecycle();
services.AddSingleton<MyProvisioningWorkflow>();
services.AddSingleton<ITenantLifecycleWorkflow>(sp => sp.GetRequiredService<MyProvisioningWorkflow>());

// 3. Map the status and resume endpoints.
app.MapPlatformTenantLifecycleStatus();
app.MapPlatformTenantLifecycleResume();
app.AddPlatformTenantLifecycleReadiness();
```

## Migration from the starter provisioning workflow

1. Implement `ITenantLifecycleWorkflow` for each existing provisioning flow
   (the application keeps its existing `Provisioning` class, the platform only
   asks for an ordered list of steps with stable names and idempotency
   guarantees).
2. Persist an `ITenantLifecycleStore` row per provisioning operation. The
   starter catalog tables can back the store unchanged; the platform never
   reads them directly.
3. Wire `ITenantLifecycleScopeCallback` to `ITenantScopeFactory` so the
   multitenancy adapter installs the ambient scope around each tenant-scoped
   step.
4. Rollback: remove `AddPlatformTenantLifecycle` and the endpoint mappings.
   The starter's `Provisioning` and `TenantExpiryScanJob` continue to work
   because the platform never mutates the catalog, the connection policy, or
   the migration process.

## Security

- Step names are public identifiers; do not embed tenant secrets or connection
  strings.
- `TenantLifecycleStepResult.SafeMessage` MUST NOT contain provider exception
  text, stack traces, or connection strings. The platform reports the safe
  message verbatim to readiness, status, and resume endpoints.
- The orchestrator refuses to transition a terminal operation to a different
  state; the `InMemoryTenantLifecycleStore` enforces the same rule.
