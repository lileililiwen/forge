# Tasks

## 1. Project and packaging
- [x] Add `src/Platform.RateLimiting/Platform.RateLimiting.csproj` targeting `net8.0` with `<PackageReference>` entries for `Microsoft.Extensions.Options` and `Microsoft.Extensions.DependencyInjection.Abstractions` only.
- [x] Add the new project to `Platform.sln`, `Directory.Build.props` packaging metadata, and `Directory.Packages.props` with version `0.1.0`.
- [x] Set `<Nullable>enable</Nullable>`, `<ImplicitUsings>enable</ImplicitUsings>`, and `<TreatWarningsAsErrors>true</TreatWarningsAsErrors>` in the project file.

## 2. Types and registration
- [x] Move `IRateLimiter`, `InMemoryRateLimiter`, `RateLimitDecision`, `RateLimitKey`, `RateLimitPolicies`, `IRateLimitBypassResolver`, `IRateLimiterBackendStatusProvider`, `IRateLimiterBackendStatus`, `HttpContextAbstraction`, `RateLimitingOptions`, and `RateLimitPolicyOptions` into the new package under the `Platform.RateLimiting` namespace.
- [x] Refactor `InMemoryRateLimiter` to consume `IClock` and to read `RateLimitPolicies` from `IOptions<RateLimitingOptions>`.
- [x] Add `AddPlatformRateLimiting` extension with the documented default policy catalog; preserve the existing public surface and metric names.

## 3. Tests
- [x] Add unit tests for the windowed counter, the bypass resolver, the in-memory backend status, and the configuration validation.
- [x] Add a TestServer integration test that proves the registration, the documented default policy catalog, and the readiness surface.
- [x] Extend `Platform.Architecture.Tests` to forbid `Platform.RateLimiting` from referencing ASP.NET Core, EF Core, StackExchange.Redis, or a VisualFlow project.

## 4. Verification
- [x] Run `dotnet restore Platform.sln`, `dotnet build Platform.sln -c Release --no-restore`, `dotnet test Platform.sln -c Release --no-build --nologo`, and `dotnet pack src/Platform.RateLimiting/Platform.RateLimiting.csproj -c Release --no-build --nologo`.
- [x] Inspect the produced `.nupkg` and confirm `<dependencies>` contains only the documented platform contracts and the two `Microsoft.Extensions.*` abstractions.
- [x] Run `openspec validate 2026-09-08-platform-extraction-ratelimiting --strict --type change` and `openspec validate platform-ratelimiting --strict --type spec`.
