## 1. Package boundary

- [x] 1.1 Create the independently packable observability package and central package references without changing `Platform.Core`.
- [x] 1.2 Define options, service identity, correlation, redaction, activity-source, meter, and provider-status contracts.
- [x] 1.3 Adapt safe behavior from `dotnet-starter-kit/src/BuildingBlocks/Web/Observability/` rather than copying FSH namespaces, module meter names, or product claims.

## 2. Runtime implementation

- [x] 2.1 Implement opt-in logging/correlation registration with `TryAdd` replacement seams.
- [x] 2.2 Implement tracing and metrics registration with optional ASP.NET Core, HTTP, runtime, EF, database, and cache instrumentation.
- [x] 2.3 Implement redaction and bounded-label policies for headers, routes, tenant IDs, cache keys, and provider diagnostics.
- [x] 2.4 Add health/provider-status integration that reports safe availability without secrets or response bodies.

## 3. Verification and documentation

- [x] 3.1 Add unit tests for options, redaction, bounded labels, correlation propagation, and disabled-provider behavior.
- [x] 3.2 Add an architecture test proving `Platform.Core` remains free of ASP.NET Core, OpenTelemetry, and logging-provider dependencies.
- [x] 3.3 Add sample registration and migration guidance based on the starter references listed in `design.md`.
- [x] 3.4 Run package tests, `git diff --check`, and strict OpenSpec validation.
