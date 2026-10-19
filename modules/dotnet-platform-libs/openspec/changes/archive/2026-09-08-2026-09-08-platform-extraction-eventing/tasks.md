# Tasks

## 1. Project and packaging
- [x] Add `src/Platform.Eventing/Platform.Eventing.csproj` targeting `net8.0` with `<PackageReference>` entries for `System.Threading.Channels`, `Microsoft.Extensions.DependencyInjection.Abstractions`, and `Microsoft.Extensions.Logging.Abstractions` only.
- [x] Add the new project to `Platform.sln`, `Directory.Build.props` packaging metadata, and `Directory.Packages.props` with version `0.1.0`.
- [x] Set `<Nullable>enable</Nullable>`, `<ImplicitUsings>enable</ImplicitUsings>`, and `<TreatWarningsAsErrors>true</TreatWarningsAsErrors>` in the project file.

## 2. Types and registration
- [x] Move `IEventBus`, `IIntegrationEvent`, `IntegrationEvent`, `IntegrationEventEnvelope`, `IntegrationEventEnvelopeSerializer`, `IIntegrationEventEnvelopeDeserializer`, `IIntegrationEventHandler<TEvent>`, and `InProcessEventBus` into the new package under the `Platform.Eventing` namespace.
- [x] Refactor `IntegrationEventEnvelopeSerializer` to consume `IClock` for the default `OccurredAt` value.
- [x] Refactor `InProcessEventBus` to consume `IClock` and to use the documented `Channel<T>` bounded capacity.
- [x] Add `AddPlatformEventing` and `AddPlatformEventingInProcess` extensions; preserve the existing envelope field names.

## 3. Tests
- [x] Add unit tests for the envelope shape, the serializer round-trip, the deserializer contract, the in-process bus's bounded capacity, and the configuration validation.
- [x] Add a TestServer integration test that proves the registration, the default envelope, and the consumer dispatch.
- [x] Extend `Platform.Architecture.Tests` to forbid `Platform.Eventing` from referencing ASP.NET Core, EF Core, RabbitMQ, or a VisualFlow project.

## 4. Verification
- [x] Run `dotnet restore Platform.sln`, `dotnet build Platform.sln -c Release --no-restore`, `dotnet test Platform.sln -c Release --no-build --nologo`, and `dotnet pack src/Platform.Eventing/Platform.Eventing.csproj -c Release --no-build --nologo`.
- [x] Inspect the produced `.nupkg` and confirm `<dependencies>` contains only the documented platform contracts and the three `Microsoft.Extensions.*` abstractions (the in-box `System.Threading.Channels` ships with `net8.0` and is not a NuGet dependency).
- [x] Run `openspec validate 2026-09-08-platform-extraction-eventing --strict --type change` and `openspec validate platform-eventing --strict --type spec`.
