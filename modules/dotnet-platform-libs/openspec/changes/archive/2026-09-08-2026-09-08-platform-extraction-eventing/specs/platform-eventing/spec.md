# platform-eventing Specification

## Purpose

Provide a transport-agnostic event-bus contract, a documented envelope shape, and an in-process default adapter that any application in the portfolio can adopt without bringing in ASP.NET Core, EF Core, or a specific messaging transport. The package is the third Phase 3 adoption proof: a real consumer (VisualFlow) contributes the type surface, the platform owns the package, and the consumer migrates to it in a follow-up change.

## ADDED Requirements

### Requirement: Eventing SHALL expose a documented contract

The package SHALL expose an `IEventBus` contract, an `IntegrationEvent` base record, an `IntegrationEventEnvelope` shape, a deserializer contract, and a consumer contract with `ConsumerName` and `HandleAsync` semantics.

#### Scenario: Publisher publishes a typed event

- GIVEN a consumer constructs an `IntegrationEvent` and wraps it in an `IntegrationEventEnvelope`
- WHEN the consumer calls `IEventBus.PublishAsync`
- THEN the envelope is dispatched to the registered in-process bus
- AND the envelope's `PayloadType` matches the runtime type of the event

#### Scenario: Consumer handles a typed event

- GIVEN a consumer registers an `IIntegrationEventHandler<TEvent>` with a documented `ConsumerName`
- WHEN the bus dispatches an envelope whose `PayloadType` matches `TEvent`
- THEN the consumer's `HandleAsync` is invoked with the deserialised event and the original envelope
- AND the `ConsumerName` is included in the documented metric for audit

### Requirement: Envelope SHALL be transport-agnostic

The `IntegrationEventEnvelope` SHALL carry the documented fields (`MessageId`, `PayloadType`, `PayloadJson`, `OccurredAt`, `CorrelationId`) and SHALL NOT depend on ASP.NET Core, EF Core, RabbitMQ, or a VisualFlow project.

#### Scenario: Envelope round-trip

- GIVEN a consumer serialises a typed event into an envelope
- WHEN the consumer deserialises the envelope back into a typed event
- THEN the deserialised event equals the original event by value
- AND the documented fields are preserved across the round-trip

#### Scenario: Architecture test

- GIVEN the architecture test runs
- WHEN it inspects the package's references
- THEN the test fails if any forbidden reference is present
- AND the test passes with the documented reference list

### Requirement: In-process bus SHALL be the documented default

The package SHALL ship an in-process default bus that uses a bounded `Channel<T>`, SHALL consume `IClock` for envelope timestamps, and SHALL NOT block the publisher on a slow consumer.

#### Scenario: Bounded capacity

- GIVEN the in-process bus is configured with a bounded capacity of `1024`
- WHEN a publisher publishes faster than a consumer drains
- THEN the publisher is back-pressured through the documented `ChannelWriter.WaitToWriteAsync`
- AND a consumer failure does not crash the bus

#### Scenario: Deterministic timestamps

- GIVEN a test fixes the platform clock at a known value
- WHEN the publisher constructs an envelope
- THEN `OccurredAt` is set from the fixed clock value
- AND the test does not depend on wall-clock time

### Requirement: Serializer SHALL be replaceable

The package SHALL expose an `IIntegrationEventEnvelopeDeserializer` contract and SHALL ship a default `System.Text.Json`-backed implementation that consumers can replace.

#### Scenario: Default deserializer

- GIVEN a consumer publishes an event
- WHEN a consumer handler receives the envelope
- THEN the default deserializer turns the envelope into a typed event
- AND the deserialised event is the same instance type the publisher published

#### Scenario: Custom deserializer

- GIVEN a consumer registers a custom `IIntegrationEventEnvelopeDeserializer`
- WHEN a consumer handler receives the envelope
- THEN the custom deserializer is used
- AND the bus dispatches the deserialised event to the consumer

### Requirement: Registration SHALL be opt-in

The package SHALL expose `AddPlatformEventing` and `AddPlatformEventingInProcess` extension methods and SHALL NOT register unrelated persistence, logging, or transport services.

#### Scenario: Minimal host adoption

- GIVEN a host registers the platform eventing services
- WHEN the host starts
- THEN only the documented platform services are added
- AND no messaging transport, persistence, or logging provider is registered

#### Scenario: In-process adapter

- GIVEN a host registers `AddPlatformEventingInProcess`
- WHEN the host starts
- THEN the in-process bus is registered
- AND consumers registered through `IIntegrationEventHandler<TEvent>` are wired to the bus
