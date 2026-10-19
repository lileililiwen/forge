## Why

The starter kit has a reusable host observability stack, but the platform currently exposes only scattered telemetry hooks. Consumers otherwise reimplement correlation enrichment, OpenTelemetry registration, structured logging, and provider-safe diagnostics.

## What Changes

- Add an independently adoptable observability package for ASP.NET Core, HTTP client, runtime, database, cache, and platform activity/metric registration.
- Define safe correlation, service identity, exporter, and redaction options.
- Keep provider registration opt-in and avoid product/module meter names in the package.
- Add deterministic tests, documentation, and a sample integration.

## Capabilities

### New Capabilities

- `platform-observability`: Structured logging, tracing, metrics, correlation, and safe diagnostics for host applications.

### Modified Capabilities

- None.

## Impact

Adds a new optional ASP.NET Core/hosting package and public options/contracts. `Platform.Core` and framework-neutral packages remain unchanged. No application data, log schema, exporter endpoint, or secret is owned by the platform.
