# Platform.Web

`Platform.Web` is an explicit, provider-neutral runtime layer on top of
`Platform.AspNetCore`. It does not register authentication, persistence, or
external providers.

```csharp
builder.Services.AddPlatformWeb(options =>
{
    options.MaxRequestBodyBytes = 2 * 1024 * 1024;
    options.ReadinessPath = "/ready";
});

var app = builder.Build();
app.UsePlatformWeb();
app.MapPlatformRuntimeEndpoints();
```

`UsePlatformWeb` installs correlation, sanitized exception handling, security
headers, request-size enforcement, and request cancellation timeout in that
order. Map runtime endpoints after middleware and after application health
checks have been registered. `/live` checks only the platform liveness path;
`/ready` composes all non-`live` health checks and registered provider status
sources, returning `503` when any dependency is unavailable.

The default correlation policy generates a fresh `X-Correlation-Id`. Hosts may
opt into incoming values, but values must be printable ASCII and fit within
`MaxCorrelationIdLength`. The default request limit is 1 MiB and the default
timeout is 30 seconds. Replace `IPlatformRedactor`,
`IPlatformConfigurationValidator`, or `IProviderStatusSource` with host-owned
implementations through normal DI registration.
