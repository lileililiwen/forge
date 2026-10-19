## Why

The applications repeat technical concepts such as time access, operation results, audit metadata, and caller identity. These concepts can be shared without sharing application domain entities.

## What Changes

- Add dependency-light core contracts.
- Add explicit result and error types.
- Add time and audit abstractions.
- Add caller/tenant context contracts without persistence behavior.

## Capabilities

### New Capabilities

- `platform-core-contracts`: Stable, framework-independent technical contracts for consuming applications.

### Modified Capabilities

## Impact

Adds public APIs to `Platform.Core`. Consumers may reference them without taking a dependency on ASP.NET Core or EF Core.

## Context

The existing applications use different domain models and frameworks. Only semantics that are demonstrably technical and stable should be shared.

## Goals / Non-Goals

**Goals:**

- Provide small interfaces and records that can be implemented by each application.
- Make time-dependent behavior testable.
- Avoid a universal base entity.

**Non-Goals:**

- Define EF Core entities or migrations.
- Define a universal tenant model.
- Move application domain models into the platform.

## Decisions

- Prefer interfaces and records over inheritance-heavy base classes.
- Use UTC-oriented time contracts.
- Use structured error codes rather than framework-specific exceptions in core contracts.

## Risks / Trade-offs

- Shared contracts become expensive to change after adoption; version them conservatively.
- A minimal core may initially leave small amounts of duplicate application code, which is preferable to premature coupling.
