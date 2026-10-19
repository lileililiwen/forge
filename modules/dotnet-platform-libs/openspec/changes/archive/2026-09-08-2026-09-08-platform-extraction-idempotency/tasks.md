# Tasks

## 1. Project and packaging
- [x] Add `src/Platform.Idempotency/Platform.Idempotency.csproj` targeting `net8.0` with `<PackageReference>` entries for `Microsoft.Extensions.Options` and `Microsoft.Extensions.DependencyInjection.Abstractions` only.
- [x] Add the new project to `Platform.sln`, `Directory.Build.props` packaging metadata, and `Directory.Packages.props` with version `0.1.0`.
- [x] Set `<Nullable>enable</Nullable>`, `<ImplicitUsings>enable</ImplicitUsings>`, and `<TreatWarningsAsErrors>true</TreatWarningsAsErrors>` in the project file.

## 2. Types and registration
- [x] Move `IdempotencyRecord`, `IIdempotencyStore`, `InMemoryIdempotencyStore`, `RequestFingerprint`, `IdempotencyOptions`, and `IdempotencyMetrics` into the new package under the `Platform.Idempotency` namespace.
- [x] Refactor `InMemoryIdempotencyStore` to consume `IClock` and to honour the documented `RetentionSeconds` and `MaxKeyLength`.
- [x] Add `AddPlatformIdempotency` extension and preserve the existing metric names.

## 3. Tests
- [x] Add unit tests for `TryGetAsync`, `SaveAsync`, `EvictExpiredAsync`, the fingerprint helper, and the configuration validation.
- [x] Add a TestServer integration test that proves the registration, the documented default retention window, and the eviction sweep.
- [x] Extend `Platform.Architecture.Tests` to forbid `Platform.Idempotency` from referencing ASP.NET Core, EF Core, StackExchange.Redis, or a VisualFlow project.

## 4. Verification
- [x] Run `dotnet restore Platform.sln`, `dotnet build Platform.sln -c Release --no-restore`, `dotnet test Platform.sln -c Release --no-build --nologo`, and `dotnet pack src/Platform.Idempotency/Platform.Idempotency.csproj -c Release --no-build --nologo`.
- [x] Inspect the produced `.nupkg` and confirm `<dependencies>` contains only the documented platform contracts and the two `Microsoft.Extensions.*` abstractions.
- [x] Run `openspec validate 2026-09-08-platform-extraction-idempotency --strict --type change` and `openspec validate platform-idempotency --strict --type spec`.
