# Proposal: Adopt VisualFlow's eventing contracts and in-process adapter into the platform

## Why

Every consumer in the portfolio that wants decoupled modules ships an event bus with the same shape: an `IEventBus` contract, an `IntegrationEvent` marker, an envelope that carries the payload type and JSON body, a deserializer that turns the envelope back into a typed event, a consumer handler with a `ConsumerName` and an `HandleAsync`, and an in-process default adapter for tests and single-host deployments. VisualFlow has implemented this in `src/BuildingBlocks/Eventing/` (contracts) and `src/BuildingBlocks/Eventing.InProcess/` (in-memory adapter), and both projects are framework-neutral. Promoting them gives the portfolio a shared, transport-agnostic event contract that any consumer can adopt at its own pace, and proves that the platform can host a richer surface (contracts + an in-process adapter in one package) without leaking implementation.

## What Changes

- Add a new production package `Platform.Eventing` that owns the event contracts, envelope shape, deserializer, and the in-process default bus.
- The package depends on `Platform.Core` for `IClock` and the documented `Error` shape; it does not depend on ASP.NET Core, EF Core, RabbitMQ, or any messaging transport.
- VisualFlow's `src/BuildingBlocks/Eventing/` and `src/BuildingBlocks/Eventing.InProcess/` are removed in a follow-up VisualFlow change that adopts the new package; this change is the platform side only.

## Capabilities

### New Capabilities

- `platform-eventing`: Documented `IEventBus`, `IntegrationEvent`, `IntegrationEventEnvelope`, deserializer contract, and an in-process default adapter with `ConsumerName` and `HandleAsync` semantics.

### Modified Capabilities

- (none)

## Impact

- Adds one new production NuGet package, `Platform.Eventing`, versioned in `Directory.Packages.props`.
- Expands the architecture test to forbid `Platform.Eventing` from referencing ASP.NET Core, EF Core, RabbitMQ, or a VisualFlow project.
- VisualFlow adopts the package in a separate change after this lands; no VisualFlow code is modified in this change.

## Context

The platform roadmap's Phase 3 adoption proof needs at least one shared contract that crosses module boundaries. The VisualFlow eventing block is the natural choice because the contract (`IEventBus`, `IntegrationEvent`, `IntegrationEventEnvelope`, `IIntegrationEventHandler<T>`, `IIntegrationEventEnvelopeDeserializer`) is the same shape every consumer needs, and the in-process adapter is the default for tests and single-host deployments.

## Goals / Non-Goals

**Goals:**

- Ship a self-contained, transport-agnostic eventing package.
- Preserve the documented envelope shape (`PayloadType`, `PayloadJson`, `OccurredAt`, `CorrelationId`) so a consumer can swap in a RabbitMQ or Service Bus adapter without changing the call sites.
- Provide a documented in-process default bus that consumers can use in tests and single-host deployments.

**Non-Goals:**

- Ship a RabbitMQ or Service Bus adapter in this change; a follow-up `platform-eventing-rabbitmq` change can adopt one if a second consumer needs it.
- Provide an outbox or inbox in this change; that is the consumer's responsibility and lives in `Platform.RateLimiting`'s sibling adoption changes.
- Replace the existing in-process bus in any consumer.

## Decisions

- Reuse the existing `Platform.Core.IClock` for envelope timestamp generation.
- Use the `IntegrationEventEnvelope` shape verbatim from VisualFlow; the documented field names are preserved.
- Keep the `IIntegrationEventEnvelopeDeserializer` as part of the public contract; this lets consumers plug in a custom JSON shape (for example, with `System.Text.Json` source generators) without changing the bus.
- The new package is added to `Directory.Build.props` packaging metadata with a stable `0.1.0` version.

## Risks / Trade-offs

- The in-process bus uses a single-process `Channel<T>`; cross-process delivery needs a transport adapter. This is documented as a non-goal and is the consumer's responsibility.
- The `IntegrationEventEnvelopeSerializer` uses `System.Text.Json`; consumers that need a different serializer (for example, with attribute-based polymorphism) can replace it via the documented deserializer contract.
