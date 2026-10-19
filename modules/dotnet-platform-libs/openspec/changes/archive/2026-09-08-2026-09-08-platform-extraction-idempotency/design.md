# Design

## Dependencies

This change depends on `platform-core-contracts` for `IClock`. It does not depend on ASP.NET Core, EF Core, Redis, or a payment provider.

## Components

Add a `src/Platform.Idempotency/Platform.Idempotency.csproj` project targeting `net8.0`. The project declares `<PackageReference Include="Microsoft.Extensions.Options" />` and `<PackageReference Include="Microsoft.Extensions.DependencyInjection.Abstractions" />` as transitive dependencies only.

Move the following types from `src/VisualFlow.BuildingBlocks.Idempotency/` into `src/Platform.Idempotency/` under the `Platform.Idempotency` namespace, preserving the XML doc comments and the public surface:

- `IdempotencyRecord` — the documented value type, with `Key`, `Fingerprint`, `StatusCode`, `ContentType`, `ResponseHeaders`, `ResponseBody`, and `CreatedAt`.
- `IIdempotencyStore` — the contract with `TryGetAsync`, `SaveAsync`, and `EvictExpiredAsync`.
- `InMemoryIdempotencyStore` — the default backend, refactored to consume `IClock` and to honour the documented `RetentionSeconds` and `MaxKeyLength` from `IOptions<IdempotencyOptions>`.
- `RequestFingerprint` — the documented request-fingerprint helper.
- `IdempotencyOptions` — the configuration type, with `Enabled`, `Storage`, `RetentionSeconds`, `MaxKeyLength`, and `HeaderName`.
- `IdempotencyServiceCollectionExtensions` — the opt-in registration, renamed to `AddPlatformIdempotency`.
- `IdempotencyMetrics` — the documented counter surface, preserved verbatim so consumers can keep their existing dashboards.

## Compatibility

The `Microsoft.Extensions.Options` and `Microsoft.Extensions.DependencyInjection.Abstractions` references are the same versions already used in `Platform.Core`. The package is added to `Directory.Build.props` packaging metadata with the documented version `0.1.0`.

## Verification

Unit tests cover the in-memory store's `TryGetAsync`, `SaveAsync`, and `EvictExpiredAsync` paths, the fingerprint helper, and the configuration validation. The architecture test `Platform.Architecture.Tests` is extended to enforce that `Platform.Idempotency` does not reference ASP.NET Core, EF Core, StackExchange.Redis, or a VisualFlow project. A TestServer integration test proves the registration, the documented default retention window, and the eviction sweep.

## Out of scope

- A Redis or Postgres backend (`platform-idempotency-redis` or `platform-idempotency-postgres`).
- An ASP.NET Core endpoint filter.
- Migration of the VisualFlow consumer (separate change).
- Migration of any other consumer in the portfolio.
