# Design

## Dependencies

This change depends on `platform-core-contracts` for `IClock` and the documented `Error` shape. It does not depend on ASP.NET Core, EF Core, RabbitMQ, or a messaging transport.

## Components

Add a `src/Platform.Eventing/Platform.Eventing.csproj` project targeting `net8.0`. The project declares `<PackageReference Include="System.Threading.Channels" />` for the in-process bus and `<PackageReference Include="Microsoft.Extensions.DependencyInjection.Abstractions" />` and `<PackageReference Include="Microsoft.Extensions.Logging.Abstractions" />` for opt-in registration. `System.Text.Json` is the default JSON shape, exposed through the documented `IIntegrationEventEnvelopeDeserializer` contract.

Move the following types from `src/VisualFlow.BuildingBlocks.Eventing/` and `src/VisualFlow.BuildingBlocks.Eventing.InProcess/` into `src/Platform.Eventing/` under the `Platform.Eventing` namespace, preserving the XML doc comments and the public surface:

- `IEventBus` — the contract with `PublishAsync(IntegrationEventEnvelope, CancellationToken)`.
- `IIntegrationEvent` — the marker interface for typed events.
- `IntegrationEvent` — the base record carrying `EventId`, `OccurredAt`, and `CorrelationId`.
- `IntegrationEventEnvelope` — the transport-agnostic shape with `MessageId`, `PayloadType`, `PayloadJson`, `OccurredAt`, and `CorrelationId`.
- `IntegrationEventEnvelopeSerializer` — the default `System.Text.Json`-backed serializer, refactored to consume `IClock` for the default `OccurredAt` value.
- `IIntegrationEventEnvelopeDeserializer` — the deserializer contract.
- `IIntegrationEventHandler<TEvent>` — the consumer contract with `ConsumerName` and `HandleAsync(TEvent, IntegrationEventEnvelope, CancellationToken)`.
- `InProcessEventBus` — the default in-process adapter, refactored to consume `IClock` and to use the documented `Channel<T>` bounded capacity.
- `EventingServiceCollectionExtensions` and `InProcessEventingServiceCollectionExtensions` — the opt-in registrations, renamed to `AddPlatformEventing` and `AddPlatformEventingInProcess`.

## Compatibility

The `System.Threading.Channels` reference is the same version already used in the platform's test projects. The package is added to `Directory.Build.props` packaging metadata with the documented version `0.1.0`.

## Verification

Unit tests cover the envelope shape, the serializer round-trip, the deserializer contract, the in-process bus's bounded capacity, and the configuration validation. The architecture test `Platform.Architecture.Tests` is extended to enforce that `Platform.Eventing` does not reference ASP.NET Core, EF Core, RabbitMQ, or a VisualFlow project. A TestServer integration test proves the registration, the default envelope, and the consumer dispatch.

## Out of scope

- A RabbitMQ or Service Bus adapter (`platform-eventing-rabbitmq`).
- An outbox or inbox.
- An ASP.NET Core endpoint.
- Migration of the VisualFlow consumer (separate change).
- Migration of any other consumer in the portfolio.
