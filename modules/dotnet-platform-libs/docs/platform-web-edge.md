# Platform.Web edge packages

Four small, independently adoptable packages extend `Platform.Web` with the
adjacent capabilities the starter kit otherwise has to bring in directly. Each
package is optional; consumers adopt them one at a time, and nothing in the
core runtime changes.

| Package | Project | Purpose |
| --- | --- | --- |
| `Platform.Web.Telemetry` | `net10.0` | Framework-neutral redaction-safe telemetry names, option-validation helpers, and a structured log sink. |
| `Platform.Web.Cors` | `net10.0` | Optional ASP.NET Core CORS configuration with strict production-time validation. |
| `Platform.Web.Resilience` | `net10.0` | Optional `HttpClient` resilience (retry, timeout, circuit breaker) with bounded defaults. |
| `Platform.Web.OpenApi` | `net10.0` | Optional OpenAPI document registry and explicit JSON endpoint mapping. |

## Platform.Web.Telemetry

`Platform.Web.Telemetry` is the framework-neutral root of the edge packages.
It defines stable request and provider instrumentation names, a redactor
contract, a configurable safe-value policy, and an `IPlatformWebTelemetry` log
sink. Other edge packages take a dependency on it, but applications can
consume it directly to record stable telemetry without adopting any other
edge capability.

```csharp
services.AddPlatformWebTelemetry(options =>
{
    options.ApplicationName = "billing.api";
});
```

The package re-exports a reusable `AddValidatedOptions<T>()` extension that
replaces the per-package `AddOptions<T>().Validate(o => o.Validate().Count == 0, ...)`
pattern. Existing `Platform.Web` option wiring is unchanged.

## Platform.Web.Cors

`Platform.Web.Cors` is the optional CORS layer. It registers named CORS
policies, validates them at registration time, and exposes
`UsePlatformWebCors(policyName)`. The validator refuses unsafe combinations in
production: wildcard origins with credentials, wildcard origins in production,
non-absolute origins, and the absence of any allowed origin in production.

```csharp
services.AddPlatformWebCors(builder.Environment, options =>
{
    options.Policies.Add(new PlatformWebCorsPolicyOptions
    {
        Name = "default",
        AllowedOrigins = { "https://app.example.com" },
        AllowedMethods = { "GET", "POST" },
    });
});

app.UsePlatformWebCors();
```

Local development may continue to use `http://localhost`. Production
environments must list explicit HTTPS origins.

## Platform.Web.Resilience

`Platform.Web.Resilience` adds a `DelegatingHandler` that wraps `HttpClient`
calls with bounded retry, timeout, and circuit-breaker semantics. The default
idempotent set is `GET`, `HEAD`, and `OPTIONS`; `PUT` and `DELETE` with
`If-Match` are also retried. `POST` and `PATCH` are not retried unless the host
explicitly adds them via `PlatformHttpResilienceOptions.IdempotentMethods`.

```csharp
services.AddHttpClient("provider")
    .AddPlatformHttpResilience();
```

A `PlatformHttpCircuitOpenException` is thrown when the circuit is open. Each
attempt emits an `X-Retry-Attempt` header and a stable resilience event through
`IHttpResilienceTelemetry` (default: `DefaultHttpResilienceTelemetry` which
bridges to the platform web telemetry sink).

## Platform.Web.OpenApi

`Platform.Web.OpenApi` provides an `IPlatformOpenApiDocumentRegistry` plus
mapping helpers. The platform does not own a specific OpenAPI implementation;
applications register an `IPlatformOpenApiDocumentProvider` that supplies the
JSON for each named document. The platform never auto-discovers endpoints.

```csharp
services.AddPlatformWebOpenApi(options =>
{
    options.Documents.Add(new PlatformWebOpenApiDocumentOptions
    {
        Name = "v1",
        Path = "/openapi/v1.json",
        Title = "Billing API"
    });
});

services.AddSingleton<IPlatformOpenApiDocumentProvider, MySwashbuckleProvider>();

app.MapPlatformOpenApiDocuments();
```

Authorization metadata applied with `RequireAuthorization()` on the route is
preserved. Unknown document names resolve to `404`.

## Adoption order

1. Add `Platform.Web.Telemetry` if you need the shared telemetry names or the
   `AddValidatedOptions<T>()` helper.
2. Add one of the ASP.NET Core packages (`Cors`, `Resilience`, `OpenApi`) only
   when its capability is required.
3. Roll back by removing the package reference and its registration. No data
   migration is required.
