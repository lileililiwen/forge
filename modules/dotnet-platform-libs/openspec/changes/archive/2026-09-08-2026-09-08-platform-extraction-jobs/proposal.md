# Proposal: Adopt VisualFlow's background-jobs contracts into the platform

## Why

Every consumer in the portfolio that ships recurring work needs a documented `IJobDispatcher`, an `IRecurringJobHandler`, an `IRecurringJobRegistry`, an `IJobTelemetry` surface, an attribute-based registration, and a `RecurringJobDescriptor` shape. VisualFlow has implemented exactly this in `src/BuildingBlocks/Jobs/`, including the `RecurringJobAttribute`, the `BackgroundJobsOptions`, and the documented opt-in registration. The type surface is framework-neutral; the only `Microsoft.Extensions.*` reference is `IOptions<>`. Promoting it gives the portfolio a shared, recurring-job contract that any consumer can adopt at its own pace.

## What Changes

- Add a new production package `Platform.Jobs` that owns the recurring-job contracts, the dispatcher contract, the attribute-based registration, the descriptor shape, the options, and the opt-in registration.
- The package depends on `Platform.Core` for `IClock` and the documented `Error` shape; it does not depend on ASP.NET Core, EF Core, Hangfire, Quartz, or a specific scheduling engine.
- VisualFlow's `src/BuildingBlocks/Jobs/` is removed in a follow-up VisualFlow change that adopts the new package; this change is the platform side only.

## Capabilities

### New Capabilities

- `platform-jobs`: Documented `IJobDispatcher`, `IRecurringJobHandler`, `IRecurringJobRegistry`, `IJobTelemetry`, `RecurringJobAttribute`, `RecurringJobDescriptor`, `BackgroundJobsOptions`, and opt-in registration.

### Modified Capabilities

- (none)

## Impact

- Adds one new production NuGet package, `Platform.Jobs`, versioned in `Directory.Packages.props`.
- Expands the architecture test to forbid `Platform.Jobs` from referencing ASP.NET Core, EF Core, Hangfire, Quartz, or a VisualFlow project.
- VisualFlow adopts the package in a separate change after this lands; no VisualFlow code is modified in this change.

## Context

The platform roadmap's Phase 3 adoption proof needs to cover at least one scheduling-flavored contract. The VisualFlow jobs block is the natural fit: it is small, framework-neutral, has a documented attribute-based registration, and the only `Microsoft.Extensions.*` reference is `IOptions<>`.

## Goals / Non-Goals

**Goals:**

- Ship a self-contained, engine-neutral scheduling package.
- Preserve the documented `RecurringJobAttribute` so consumers can annotate their handlers without changing the call sites.
- Provide a documented `IJobTelemetry` surface that consumers can wire to their existing observability layer.

**Non-Goals:**

- Ship a Hangfire, Quartz, or hosted-service scheduler in this change; a follow-up `platform-jobs-hosted` change can adopt one if a second consumer needs it.
- Ship a dashboard; that is the consumer's responsibility.
- Replace the existing scheduling implementation in any consumer.

## Decisions

- Reuse the existing `Platform.Core.IClock` for the documented `RecurringJobDescriptor` and the dispatcher contract so the contract is deterministic and testable.
- Use the `BackgroundJobsOptions` shape verbatim from VisualFlow; the documented section name (`BackgroundJobs`) is preserved.
- Keep the `RecurringJobAttribute` as a runtime attribute so consumers can decorate their handlers with a documented `Cron` expression.

## Risks / Trade-offs

- The `IJobDispatcher` contract is intentionally minimal; consumers can implement it against Hangfire, Quartz, or a hosted service without changing the call sites.
- The `IRecurringJobRegistry` is a contract only; the platform does not ship a default implementation. This is documented in the consumer guide and in the spec.
