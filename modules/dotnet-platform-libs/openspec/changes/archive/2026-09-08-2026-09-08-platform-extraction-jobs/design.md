# Design

## Dependencies

This change depends on `platform-core-contracts` for `IClock` and the documented `Error` shape. It does not depend on ASP.NET Core, EF Core, Hangfire, Quartz, or a scheduling engine.

## Components

Add a `src/Platform.Jobs/Platform.Jobs.csproj` project targeting `net8.0`. The project declares `<PackageReference Include="Microsoft.Extensions.Options" />` and `<PackageReference Include="Microsoft.Extensions.DependencyInjection.Abstractions" />` as transitive dependencies only.

Move the following types from `src/VisualFlow.BuildingBlocks.Jobs/` into `src/Platform.Jobs/` under the `Platform.Jobs` namespace, preserving the XML doc comments and the public surface:

- `IJobDispatcher` — the contract with `EnqueueAsync` and the documented payload shape.
- `IRecurringJobHandler` — the marker interface for recurring handlers.
- `IRecurringJobRegistry` — the contract with `Register(RecurringJobDescriptor)`.
- `IJobTelemetry` — the documented telemetry surface.
- `RecurringJobAttribute` — the runtime attribute carrying the documented `Cron` expression and the documented `Name`.
- `RecurringJobDescriptor` — the value type with `Name`, `Cron`, `HandlerType`, and `Options`.
- `BackgroundJobsOptions` — the configuration type with the documented fields.
- `BackgroundJobsServiceCollectionExtensions` — the opt-in registration, renamed to `AddPlatformJobs`.

## Compatibility

The `Microsoft.Extensions.Options` and `Microsoft.Extensions.DependencyInjection.Abstractions` references are the same versions already used in `Platform.Core`. The package is added to `Directory.Build.props` packaging metadata with the documented version `0.1.0`.

## Verification

Unit tests cover the `RecurringJobAttribute` reflection, the `RecurringJobDescriptor` construction, the `IJobTelemetry` surface, and the configuration validation. The architecture test `Platform.Architecture.Tests` is extended to enforce that `Platform.Jobs` does not reference ASP.NET Core, EF Core, Hangfire, Quartz, or a VisualFlow project. A TestServer integration test proves the registration and the documented default options.

## Out of scope

- A Hangfire or Quartz adapter (`platform-jobs-hangfire` or `platform-jobs-quartz`).
- A hosted-service scheduler.
- A dashboard.
- Migration of the VisualFlow consumer (separate change).
- Migration of any other consumer in the portfolio.
