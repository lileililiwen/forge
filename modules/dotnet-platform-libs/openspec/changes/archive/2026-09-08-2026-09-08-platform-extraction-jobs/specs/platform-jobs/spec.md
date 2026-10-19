# platform-jobs Specification

## Purpose

Provide an engine-neutral scheduling contract, a documented `RecurringJobAttribute`, a dispatcher contract, a registry contract, a telemetry surface, and an opt-in registration that any application in the portfolio can adopt without bringing in ASP.NET Core, EF Core, Hangfire, Quartz, or a specific scheduling engine. The package is the fifth Phase 3 adoption proof: a real consumer (VisualFlow) contributes the type surface, the platform owns the package, and the consumer migrates to it in a follow-up change.

## ADDED Requirements

### Requirement: Jobs SHALL expose a documented contract and registration surface

The package SHALL expose an `IJobDispatcher` contract, an `IRecurringJobHandler` marker interface, an `IRecurringJobRegistry` contract, an `IJobTelemetry` surface, a `RecurringJobAttribute` runtime attribute, a `RecurringJobDescriptor` value type, a `BackgroundJobsOptions` configuration type, and an opt-in registration extension.

#### Scenario: Consumer registers a recurring job

- GIVEN a consumer decorates a handler with `RecurringJobAttribute("0 * * * *", Name = "...")`
- WHEN the consumer registers the handler through `IRecurringJobRegistry.Register`
- THEN a `RecurringJobDescriptor` is registered with the documented `Cron` and `Name`
- AND the descriptor is observable through the documented telemetry surface

#### Scenario: Consumer dispatches a job

- GIVEN a consumer calls `IJobDispatcher.EnqueueAsync` with a documented payload
- WHEN the dispatcher validates the payload
- THEN the dispatcher accepts the payload
- AND the consumer can opt out without changing the call sites

### Requirement: Attribute SHALL be runtime-discoverable

`RecurringJobAttribute` SHALL be a runtime attribute with a documented `Cron` expression and a documented `Name`; the package SHALL expose a documented reflection helper that returns the `RecurringJobDescriptor` for a decorated handler.

#### Scenario: Reflection helper

- GIVEN a consumer decorates a handler with `RecurringJobAttribute("0 * * * *", Name = "presence-evict")`
- WHEN the consumer calls the documented reflection helper
- THEN the helper returns a `RecurringJobDescriptor` with the documented `Cron` and `Name`

#### Scenario: Missing attribute

- GIVEN a consumer calls the reflection helper on a handler without `RecurringJobAttribute`
- WHEN the helper validates the handler
- THEN the helper throws the documented `InvalidOperationException`
- AND no descriptor is registered

### Requirement: Telemetry SHALL be observable

`IJobTelemetry` SHALL expose a documented surface that consumers can wire to their existing observability layer; the package SHALL NOT ship a default implementation.

#### Scenario: Consumer wires telemetry

- GIVEN a consumer registers a custom `IJobTelemetry`
- WHEN a recurring job is registered
- THEN the telemetry surface is invoked with the documented payload
- AND the consumer can opt out without changing the call sites

#### Scenario: No default telemetry

- GIVEN a consumer has not registered a custom `IJobTelemetry`
- WHEN the host starts
- THEN the package does not register a default implementation
- AND the consumer can opt in to telemetry without changing the call sites

### Requirement: Options SHALL be configurable

The package SHALL expose a `BackgroundJobsOptions` configuration type and SHALL bind it to the documented `BackgroundJobs` configuration section.

#### Scenario: Default registration

- GIVEN a consumer calls `AddPlatformJobs`
- WHEN the host starts
- THEN `BackgroundJobsOptions` is bound to the `BackgroundJobs` configuration section
- AND the documented defaults are applied when no section is present

#### Scenario: Consumer overrides defaults

- GIVEN a consumer sets a documented option in configuration
- WHEN the host starts
- THEN the configured value overrides the documented default
- AND the consumer can opt out without changing the call sites

### Requirement: Package SHALL be engine-neutral and framework-neutral

The package SHALL depend only on `Platform.Core`, `Microsoft.Extensions.Options`, and `Microsoft.Extensions.DependencyInjection.Abstractions`. It SHALL NOT reference ASP.NET Core, EF Core, Hangfire, Quartz, or a VisualFlow project.

#### Scenario: Architecture test

- GIVEN the architecture test runs
- WHEN it inspects the package's references
- THEN the test fails if any forbidden reference is present
- AND the test passes with the documented reference list

### Requirement: Clock access SHALL come from Platform.Core

The dispatcher and the registry SHALL read the current time from `IClock` and SHALL NOT call `DateTimeOffset.UtcNow` directly.

#### Scenario: Deterministic test

- GIVEN a test fixes the platform clock at a known value
- WHEN the consumer dispatches a job
- THEN the dispatcher is computed from the fixed clock value
- AND the test does not depend on wall-clock time
