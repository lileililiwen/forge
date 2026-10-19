## Context

The starter kit combines EF outbox/inbox storage with `RabbitMqEventBus`, `OutboxDispatcher`, tenant scopes, leases, retries, and dead-letter handling. The platform durable eventing documentation explicitly leaves the transport publisher to the application.

Starter-kit references:

- `dotnet-starter-kit/src/BuildingBlocks/Eventing/RabbitMq/RabbitMqEventBus.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Eventing/RabbitMq/RabbitMqOptions.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Eventing/Outbox/OutboxDispatcher.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Eventing/Inbox/EfCoreInboxStore.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Eventing/Serialization/JsonEventSerializer.cs`

Implementation agents may copy connection, confirmation, retry, and serialization patterns, but must adapt them to `DurableEventEnvelope`, `IDurableEventPublisher`, and application-owned topology. Do not copy starter event names or module assemblies.

## Goals / Non-Goals

**Goals:**

- Provide a replaceable RabbitMQ implementation of the existing durable publisher seam.
- Support publisher confirms, bounded waits, cancellation, safe failure classification, and explicit retry/dead-letter behavior.
- Keep transport registration optional and independently packable.

**Non-Goals:**

- Owning EF migrations, queue/exchange names, event schemas, credentials, or replay tools.
- Changing at-least-once delivery semantics.
- Adding RabbitMQ dependencies to framework-neutral eventing contracts.

## Decisions

1. Put RabbitMQ code in `Platform.Eventing.RabbitMq`; reference only platform eventing contracts and the RabbitMQ client.
2. Implement `IDurableEventPublisher` and expose registration options for connection, topology prefix, confirm timeout, and serialization policy.
3. Use application-provided topology/type registration. Reject unregistered event types rather than publishing ambiguous payloads.
4. Treat publisher confirmation and connection failures as transient; malformed configuration and unsupported payloads as permanent/configuration failures.

Alternative rejected: moving RabbitMQ into `Platform.Eventing` would make every consumer pay the transport dependency and violate independent adoption.

## Risks / Trade-offs

- [Risk] At-least-once delivery duplicates side effects → document idempotent consumers and rely on platform inbox contracts.
- [Risk] Topology drift across applications → require explicit topology names/versioning from the host.
- [Risk] Broker outages delay outbox draining → return safe transient failures and preserve durable retry state.

## Migration Plan

Wrap the starter publisher behind the platform interface, run in-process tests, then enable RabbitMQ in one environment. Disable the adapter and hosted dispatcher to roll back; durable rows remain application-owned.

## Open Questions

- Whether consumer-side hosted delivery belongs in this package or remains an application-specific adapter around the platform publisher.
