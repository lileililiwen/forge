# Design

## Dependencies

This change depends on `platform-core-contracts` for `IClock` and the documented `Error` shape. It does not depend on ASP.NET Core, EF Core, or any rate-limit provider.

## Components

Add a `src/Platform.RateLimiting/Platform.RateLimiting.csproj` project targeting `net8.0`. The project declares `<PackageReference Include="Microsoft.Extensions.Options" />` and `<PackageReference Include="Microsoft.Extensions.DependencyInjection.Abstractions" />` as transitive dependencies only — both are already in `Directory.Packages.props`. The package is added to `Directory.Build.props` packaging metadata with the documented version `0.1.0`.

Move the following types from `src/VisualFlow.BuildingBlocks.RateLimiting/` into `src/Platform.RateLimiting/` under the `Platform.RateLimiting` namespace, preserving the XML doc comments and the public surface:

- `IRateLimiter` — the contract.
- `InMemoryRateLimiter` — the in-memory default backend, refactored to consume `IClock` instead of `DateTimeOffset.UtcNow` and to read `RateLimitPolicies` from a constructor-injected `IOptions<RateLimitingOptions>`.
- `RateLimitDecision`, `RateLimitKey` — the value types returned by the limiter.
- `RateLimitPolicies` — the documented default policy catalog (`feed`, `search`, `uploads`, `downloads`, `account-recovery`). Consumers can override by setting their own `RateLimitPolicies` registration before `AddPlatformRateLimiting` is called.
- `IRateLimitBypassResolver` — the bypass-resolver contract.
- `IRateLimiterBackendStatusProvider` and `IRateLimiterBackendStatus` — the readiness surface.
- `HttpContextAbstraction` — the framework-neutral wrapper that the bypass resolver consumes.
- `RateLimitingOptions`, `RateLimitPolicyOptions` — the configuration types.
- `RateLimitingServiceCollectionExtensions` — the opt-in registration. The method is renamed to `AddPlatformRateLimiting` and the policy catalog is registered as a singleton.
- `RateLimitMetrics` — the documented counter surface; the new package exposes the same metric names so consumers can keep their existing dashboards.

## Compatibility

The `Microsoft.Extensions.Options` and `Microsoft.Extensions.DependencyInjection.Abstractions` references are the same versions already used in `Platform.Core` and `Platform.AspNetCore`. The package is netstandard2.1-compatible only at the contract level (the new project targets `net8.0`); consumers on `net8.0` adopt the package without changes to their target framework.

## Verification

Unit tests cover the limiter's windowed counter, the bypass resolver, the in-memory backend status, and the configuration validation. The architecture test `Platform.Architecture.Tests` is extended to enforce that `Platform.RateLimiting` does not reference ASP.NET Core, EF Core, StackExchange.Redis, or a VisualFlow project. A TestServer integration test proves the registration, the documented default policy catalog, and the readiness surface.

## Out of scope

- A Redis-backed backend (`platform-ratelimiting-redis`).
- An ASP.NET Core middleware.
- Migration of the VisualFlow consumer (separate change).
- Migration of any other consumer in the portfolio.
