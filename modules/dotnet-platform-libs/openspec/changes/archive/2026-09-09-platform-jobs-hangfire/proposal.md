## Why

`Platform.Jobs` defines an engine-neutral scheduling contract, but starter-kit consumers still lack a supported Hangfire adapter with tenant/user propagation, dashboard authorization, health, and telemetry. Without it, adoption requires duplicating operationally sensitive job infrastructure.

## What Changes

- Add an optional Hangfire adapter implementing the platform job contracts.
- Support PostgreSQL and memory storage through application-selected configuration.
- Propagate tenant and subject context into jobs and provide dashboard authorization/health seams.
- Preserve engine-neutral packages and application-owned job definitions.

## Capabilities

### New Capabilities

- `platform-jobs-hangfire`: Hangfire scheduler adapter for platform job contracts.

### Modified Capabilities

- None.

## Impact

Adds Hangfire and optional storage dependencies in a separate package. `Platform.Jobs` remains engine-neutral. No product recurring jobs, permissions, persistence migrations, or dashboard credentials move into the platform.
