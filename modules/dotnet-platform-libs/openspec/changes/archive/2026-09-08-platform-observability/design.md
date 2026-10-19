## Context

`dotnet-starter-kit/src/BuildingBlocks/Web` currently wires Serilog, correlation enrichment, OpenTelemetry, HTTP, Npgsql, EF, Redis, runtime, Mediator, and Hangfire instrumentation. The platform has reusable telemetry names in individual packages but no host-level registration boundary.

Starter-kit references for implementation agents:

- `dotnet-starter-kit/src/BuildingBlocks/Web/Observability/Logging/Serilog/Extensions.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Web/Observability/Logging/Serilog/HttpRequestContextEnricher.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Web/Observability/OpenTelemetry/Extensions.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Web/Observability/OpenTelemetry/OpenTelemetryOptions.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Web/Observability/OpenTelemetry/MediatorTracingBehavior.cs`

These files are reference implementations, not source-of-truth APIs. Copy behavior selectively, remove FSH/module-specific meters, and preserve the platform's provider-neutral boundaries.

## Goals / Non-Goals

**Goals:**

- Provide opt-in host registration for structured logging, tracing, metrics, correlation, and redaction.
- Keep instrumentation packages optional and independently adoptable.
- Prevent raw secrets, authorization tokens, cache keys, request bodies, and provider response bodies from being emitted by default.

**Non-Goals:**

- Owning an exporter, collector, dashboard, Serilog sink configuration, business meter, or product log schema.
- Adding OpenTelemetry or ASP.NET Core dependencies to `Platform.Core`.
- Requiring every consumer to use Serilog or OTLP.

## Decisions

1. Create `Platform.Observability` for host registration and safe contracts. Depend only on hosting/ASP.NET/OpenTelemetry/optional logging abstractions needed by the selected package.
2. Use explicit `AddPlatformObservability` options and `TryAdd` registration. Consumers can replace logger factories, exporters, redactors, and activity sources.
3. Expose stable platform activity/metric names without product-specific labels. Redact high-cardinality and secret-bearing values before enrichment.
4. Split provider instrumentation behind options or separate adapter packages so a consumer can use logging/correlation without Redis, EF, or Npgsql.

Alternatives rejected: copying the starter's all-in-one `Web` project would force unrelated dependencies; emitting every request body would risk secret leakage and unbounded cardinality.

## Risks / Trade-offs

- [Risk] Optional instrumentation creates registration complexity → document dependency matrix and test each option independently.
- [Risk] Consumer logger configuration conflicts with defaults → use `TryAdd` and expose additive enrichers only.
- [Risk] Metric cardinality grows through consumer labels → prohibit raw IDs, paths with identifiers, keys, and payloads in platform labels.

## Migration Plan

Consumers can register the package alongside existing logging and telemetry, compare correlation and health output, then remove duplicated starter registrations one concern at a time. Rollback is package removal and restoring the previous host registrations; no migration is required.

## Open Questions

- Whether Serilog support belongs in this package or a separate `Platform.Observability.Serilog` adapter should be decided during implementation without changing the provider-neutral contracts.
