# Tasks

## 1. Project and packaging
- [x] Add `src/Platform.Jobs/Platform.Jobs.csproj` targeting `net8.0` with `<PackageReference>` entries for `Microsoft.Extensions.Options` and `Microsoft.Extensions.DependencyInjection.Abstractions` only.
- [x] Add the new project to `Platform.sln`, `Directory.Build.props` packaging metadata, and `Directory.Packages.props` with version `0.1.0`.
- [x] Set `<Nullable>enable</Nullable>`, `<ImplicitUsings>enable</ImplicitUsings>`, and `<TreatWarningsAsErrors>true</TreatWarningsAsErrors>` in the project file.

## 2. Types and registration
- [x] Move `IJobDispatcher`, `IRecurringJobHandler`, `IRecurringJobRegistry`, `IJobTelemetry`, `RecurringJobAttribute`, `RecurringJobDescriptor`, and `BackgroundJobsOptions` into the new package under the `Platform.Jobs` namespace.
- [x] Preserve the documented `RecurringJobAttribute` runtime contract and the `RecurringJobDescriptor` value shape.
- [x] Add `AddPlatformJobs` extension and preserve the existing option field names.

## 3. Tests
- [x] Add unit tests for the `RecurringJobAttribute` reflection, the `RecurringJobDescriptor` construction, the `IJobTelemetry` surface, and the configuration validation.
- [x] Add a TestServer integration test that proves the registration and the documented default options.
- [x] Extend `Platform.Architecture.Tests` to forbid `Platform.Jobs` from referencing ASP.NET Core, EF Core, Hangfire, Quartz, or a VisualFlow project.

## 4. Verification
- [x] Run `dotnet restore Platform.sln`, `dotnet build Platform.sln -c Release --no-restore`, `dotnet test Platform.sln -c Release --no-build --nologo`, and `dotnet pack src/Platform.Jobs/Platform.Jobs.csproj -c Release --no-build --nologo`.
- [x] Inspect the produced `.nupkg` and confirm `<dependencies>` contains only the documented platform contracts and the two `Microsoft.Extensions.*` abstractions.
- [x] Run `openspec validate 2026-09-08-platform-extraction-jobs --strict --type change` and `openspec validate platform-jobs --strict --type spec`.
