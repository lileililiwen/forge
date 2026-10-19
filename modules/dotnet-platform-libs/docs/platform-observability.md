# Platform.Observability

Opt-in host observability package. The platform owns no exporter, collector, dashboard, Serilog sink, or product log schema; consumers add their own OpenTelemetry SDK, OTLP exporter, console exporter, or any other sink.

## Scope

- `Platform.Observability` — ASP.NET Core package with stable activity source, meter, operation, and tag names; safe value policy; redactor contract; correlation accessor and middleware; provider status source; host lifecycle hooks; and DI registration.
- No application data, log schema, exporter endpoint, or secret is owned by the platform.

## What you get

- **Stable names** — `Platform.Observability.Host`, `Platform.Observability.Requests`, `Platform.Observability.Providers` activity sources, plus the matching `Platform.Observability.Host` meter. Operations and tag keys are documented in `PlatformObservabilityNames`; do not rename them.
- **Safe value policy** — `PlatformObservabilitySafeValuePolicy` provides `BoundTag` (length-bound only) for already-safe values such as the request route or method, `RedactTag` (redact + bound) for known-sensitive values such as the provider name and error code, and `RedactOperation` for the operation label.
- **Replaceable redactor** — `IPlatformObservabilityRedactor` and `DefaultPlatformObservabilityRedactor`. Register a custom implementation before `AddPlatformObservability` to override.
- **Correlation enrichment** — `PlatformObservabilityCorrelationMiddleware` echoes the bounded correlation identifier on the response, and `HttpPlatformCorrelationAccessor` exposes it through DI.
- **Host lifecycle** — `PlatformObservabilityHostLifetime : IHostedLifecycleService` records startup and shutdown activities and metrics.
- **Provider status** — `IPlatformObservabilityProviderStatusSource` reports safe availability; replace the registration with an exporter-aware implementation when wiring OpenTelemetry.
- **TryAdd seams** — every default registration uses `TryAdd`, so consumers can replace the redactor, accessor, recorder, status source, and host lifetime before calling `AddPlatformObservability`.

## Adoption

```csharp
using Platform.Observability;
using Platform.Observability.DependencyInjection;

var builder = WebApplication.CreateBuilder(args);
builder.Services.AddPlatformObservability(options =>
{
    options.ApplicationName = "demo.api";
    options.ApplicationVersion = "1.0.0";
    options.AcceptIncomingCorrelationHeader = true;
    options.CorrelationHeader = "X-Correlation-Id";
});

var app = builder.Build();
app.UsePlatformObservability();
// app.MapPlatformRuntimeEndpoints();  // from Platform.Web
app.Run();
```

Optionally add an OpenTelemetry SDK, OTLP exporter, or console exporter in the application:

```csharp
builder.Services.AddOpenTelemetry()
    .WithTracing(t => t.AddSource(
        Platform.Observability.PlatformObservabilityNames.HostActivitySource,
        Platform.Observability.PlatformObservabilityNames.RequestActivitySource,
        Platform.Observability.PlatformObservabilityNames.ProviderActivitySource))
    .WithMetrics(m => m.AddMeter(
        Platform.Observability.PlatformObservabilityNames.HostMeter))
    .UseOtlpExporter();
```

## Migration

Add `AddPlatformObservability` alongside the existing logging and OpenTelemetry registrations, compare the emitted activities and metrics, then remove duplicated host registrations. Rollback is the removal of the package and the registration call; nothing else changes.
