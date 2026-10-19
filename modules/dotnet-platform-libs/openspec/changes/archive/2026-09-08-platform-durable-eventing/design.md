## Context

The current eventing package serializes envelopes and dispatches them through a bounded in-process channel. The starter kit and VisualFlow both contain durable outbox/inbox implementations, but those implementations are coupled to their application models and infrastructure. Consumers need a small contract boundary that can be used with their own DbContext and migrations.

## Goals / Non-Goals

**Goals:**

- Make event publication transactionally durable through an application-owned outbox.
- Make consumer processing retryable and duplicate-safe through an inbox.
- Keep contracts usable without ASP.NET Core, EF Core, RabbitMQ, or a scheduling engine.
- Reuse the starter’s state-transition and dispatcher ideas after adapting them to platform contracts.

**Non-Goals:**

- No shared event database, migrations, RabbitMQ client, Service Bus client, or hosted platform service.
- No automatic interception of every application SaveChanges call.
- No product event catalog or cross-application event schema registry.

## Decisions

- **Split contracts from persistence.** `Platform.Eventing.Contracts` contains records and interfaces; `Platform.Eventing.EfCore` supplies relational implementations. This preserves the dependency direction and lets applications implement stores themselves.
- **Use explicit processing state.** Records expose pending, leased, succeeded, retryable-failed, and dead-letter outcomes with attempt count and next-attempt time. This is more operable than a boolean processed flag.
- **Lease before dispatch.** Inbox and outbox dispatchers claim records with lease expiry and owner identifiers. A crashed worker becomes eligible after lease expiry.
- **Keep transport-neutral envelopes.** The existing envelope serializer remains the wire boundary. RabbitMQ and other transports can be separate adapters later.
- **Use application-owned time and metadata.** Dispatchers depend on `IClock`; tenant and correlation values are copied from the envelope but are not resolved by the platform.
- **Organize each package by capability.** New packages use `Contracts`, `Persistence`, `Dispatch`, `Telemetry`, and `DependencyInjection` folders. Package roots remain top-level under `src/` because each root is independently packable.

## Risks / Trade-offs

- [Risk] Database-provider differences make claiming records difficult → [Mitigation] define a narrow claim-store interface and keep SQL/model configuration in the EF adapter; test SQLite and provider-neutral behavior separately.
- [Risk] At-least-once delivery can produce duplicate side effects → [Mitigation] require inbox idempotency keys and document handler idempotency as an application responsibility.
- [Risk] A copied starter dispatcher may import hidden FSH assumptions → [Mitigation] port behavior and tests only, review every dependency, and add architecture tests.

## Migration Plan

Consumers can continue using the existing in-process bus. A pilot application adds the contracts, maps its own outbox/inbox entities, then optionally adopts the EF adapter and dispatcher. Rollback is disabling the durable dispatcher and returning to the existing bus; application tables remain application-owned.

## Open Questions

- Whether the first EF adapter should target only PostgreSQL or use relational EF Core APIs plus SQLite tests.
- Whether outbox writing is explicit only or also exposes an opt-in SaveChanges interceptor.
