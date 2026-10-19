## Context

The starter implements Hangfire registration, filters, scoped activation, stale-lock cleanup, dashboard authorization, tenant/user job parameters, and health checks.

Starter-kit references:

- `dotnet-starter-kit/src/BuildingBlocks/Jobs/Extensions.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Jobs/FshJobFilter.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Jobs/FshJobActivator.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Jobs/Services/HangfireService.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Web/Health/HangfireHealthCheck.cs`

Agents may copy lifecycle and filter patterns, but must replace `ICurrentUser`, Finbuckle, and starter permission constants with platform context seams.

## Goals / Non-Goals

**Goals:**

- Adapt `IJobDispatcher`, recurring registration, telemetry, and context propagation to Hangfire.
- Provide explicit storage and dashboard seams with safe defaults and health reporting.
- Keep registration idempotent and replaceable.

**Non-Goals:**

- Owning Hangfire dashboard credentials, authorization policy, job names, migrations, or provider connection strings.
- Adding Hangfire dependencies to `Platform.Jobs`.

## Decisions

1. Create `Platform.Jobs.Hangfire` and register adapters only when the host opts in.
2. Use an `IJobExecutionContext` bridge supplied by the application for subject and tenant restoration; do not depend on Finbuckle or a specific identity store.
3. Expose dashboard authorization as a consumer callback and health as a provider-status source.
4. Use the platform clock/telemetry contracts where possible and preserve Hangfire's retry model without silently duplicating retries.

Alternative rejected: placing Hangfire directly in `Platform.Jobs` would make the core scheduling contract non-portable.

## Risks / Trade-offs

- [Risk] Ambient context leakage between jobs → create/dispose a scope per invocation and test restoration.
- [Risk] Dashboard exposure → require explicit authorization callback and do not enable dashboard by default.
- [Risk] Retry policy duplication → document ownership between Hangfire automatic retries and application failure handling.

## Migration Plan

Register the adapter alongside the starter job service, migrate one recurring job, compare execution and tenant context, then remove duplicate registration. Rollback is disabling the adapter; Hangfire storage remains application-owned.

## Open Questions

- Whether PostgreSQL storage deserves its own adapter package or remains an option in the Hangfire package.
