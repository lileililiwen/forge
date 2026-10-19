# Feature flags and outbound HTTP resilience

Two independently adoptable integrations replace the starter kit's tenant-aware
feature gates and standard outbound HTTP resilience pipeline:

- `Platform.FeatureManagement` — opt-in ASP.NET Core feature-flag evaluation with
  application-owned context, a tenant filter, and safe endpoint gating.
- `Platform.Http.Resilience` — framework-neutral outbound `HttpClient` resilience
  (retry, timeout, circuit breaker, concurrency) with bounded defaults.

The platform owns no feature names, rollout state, billing plans, tenant records,
provider selection, or application retry policy.

## Why these are separate packages

The starter couples feature flags, resilience, and unrelated web concerns into one
`AddHeroPlatform` registration. Consumers adopt one capability without pulling in the
other, and a host can keep its existing starter registrations while it migrates
endpoint-by-endpoint.

## Adopting feature flags

### Starter today

The starter reads tenant-scoped feature state and gates endpoints with attributes or
filters that depend on starter-owned resolution. The platform package keeps the same
shape but moves ownership out of the platform:

- Feature **definitions** and **rollout rules** stay in the host's `FeatureManagement`
  configuration section.
- The **context** (tenant, subject, custom values) is supplied by an
  `IFeatureContextResolver` implemented in application code. The default resolver
  returns an empty context.
- The **tenant filter** is `PlatformTenantFeatureFilter` (alias `PlatformTenant`),
  which compares `FeatureContext.TenantId` against the per-feature `AllowedTenants`
  parameter.

### Registration

```csharp
builder.Services.AddPlatformFeatureManagement(
    builder.Configuration,
    options => options.DisabledStatusCode = StatusCodes.Status403Forbidden);

// Supply tenant/subject context from application state.
builder.Services.AddSingleton<IFeatureContextResolver, MyFeatureContextResolver>();
```

Feature configuration is read from `FeatureManagementOptions.SectionName` (default
`FeatureManagement`); the host owns that section and the rollout rules.

### Gating an endpoint

```csharp
app.MapGet("/beta", () => "beta-on")
   .RequireFeature("Beta");
```

When the feature is disabled the endpoint returns a safe `ProblemDetails` built from
`FeatureManagementOptions` (`DisabledStatusCode`, `DisabledTitle`). No flag or rollout
state is exposed.

### Migration steps

1. Register `AddPlatformFeatureManagement` alongside the existing starter registration.
2. Implement `IFeatureContextResolver` to return the current tenant/subject; verify the
   resolver is invoked during evaluation.
3. Move gate calls from starter attributes/filters to `.RequireFeature(name)` and compare
   the disabled response.
4. Once decisions match, remove the starter feature-gate registration.

## Adopting outbound HTTP resilience

### Starter today

The starter wires a standard resilience pipeline onto named clients with fixed options.
The platform package exposes the same knobs through `PlatformHttpResilienceOptions` and
registers a `DelegatingHandler` per named client.

### Registration

```csharp
builder.Services
    .AddHttpClient("payments")
    .AddPlatformHttpResilience(options =>
    {
        options.MaxRetryAttempts = 3;
        options.TotalTimeout = TimeSpan.FromSeconds(30);
    });
```

The pipeline applies, in order: total timeout, retry (exponential back-off with jitter),
per-attempt timeout, circuit breaker, and a concurrency limiter. Set
`PlatformHttpResilienceOptions.Enabled = false` to make the handler a pass-through
during migration.

### Retry ownership

The platform adapter owns the **mechanism** of retries, timeouts, circuit breaking, and
concurrency. The **policy** (limits, which methods may be replayed, per-service overrides)
is configured through `PlatformHttpResilienceOptions`. Applications own:

- provider/client selection and credentials;
- per-named-client overrides via the `Action<PlatformHttpResilienceOptions>` overload;
- deciding which non-idempotent methods to add to `IdempotentMethods` (default safe set
  is `GET`, `HEAD`, `OPTIONS`, `TRACE`).

By default the pipeline never replays a non-idempotent request, so `POST`/`PUT`/`PATCH`/
`DELETE` are not retried unless explicitly opted in. Caller cancellation is preserved and
recorded without being converted into a retry.

### Telemetry

`IPlatformHttpResilienceTelemetry.Record(PlatformHttpResilienceEvent)` receives bounded
events (`Decision`, `Operation`, `Attempt`, `StatusCode`). The default sink is a no-op;
replace it with an application-owned sink (for example one forwarding to the platform
observability meter). Events never carry request bodies, headers, or URLs.

### Migration steps

1. Add `AddPlatformHttpResilience` to one named client and compare retry/cancel/circuit
   behavior against the starter pipeline using the integration tests as a reference.
2. Verify a transient `GET` is retried to the configured limit and a transient `POST` is
   not replayed.
3. Verify circuit-breaker open and concurrency-rejected events reach your telemetry sink.
4. Roll the handler onto the remaining clients, then remove the starter resilience
   registration.

## Rollback

Both packages are opt-in. To roll back:

- Remove `AddPlatformFeatureManagement` / `.RequireFeature` and restore the prior starter
  gates. `FeatureManagementOptions` disappears with the registration.
- Remove `AddPlatformHttpResilience`; the named clients fall back to their prior policy.
  The handler is a no-op pass-through when `Enabled = false`, so partial rollout is
  reversible without code changes.
