# Platform.Observability

Opt-in host observability package. Provides:

- `PlatformObservabilityOptions` — bounded service identity, correlation, label, and enrichment options.
- `PlatformObservabilityNames` — stable activity source, meter, operation, and tag names.
- `IPlatformObservabilityRedactor` / `DefaultPlatformObservabilityRedactor` — redaction contract and `[REDACTED]` placeholder.
- `PlatformObservabilitySafeValuePolicy` — bounds and redacts tag and operation values.
- `IPlatformActivityRecorder` / `DefaultPlatformActivityRecorder` — host, request, and provider activity emission.
- `IPlatformObservabilityProviderStatusSource` / `DefaultPlatformObservabilityProviderStatusSource` — safe provider health.
- `IPlatformObservabilityProviderRecorder` / `DefaultPlatformObservabilityProviderRecorder` — provider call enrichment.
- `IPlatformCorrelationAccessor` / `HttpPlatformCorrelationAccessor` — correlation identifier accessor.
- `PlatformObservabilityCorrelationMiddleware` — bounded correlation header echo and request enrichment.
- `PlatformObservabilityHostLifetime` — host lifecycle activities and metrics.
- `AddPlatformObservability(IServiceCollection)` and the `Action<...>` overload.
- `UsePlatformObservability(IApplicationBuilder)` — adds the correlation middleware.

The platform owns no exporter, collector, dashboard, Serilog sink, or product log schema.
Consumers add their own OpenTelemetry SDK, OTLP exporter, console exporter, or any other sink.
