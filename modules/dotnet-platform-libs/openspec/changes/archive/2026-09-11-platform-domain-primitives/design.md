## Context

`Platform.Core` contains stable results, time, caller, tenancy, and audit contracts. The starter kit also exposes `BaseEntity`, aggregate/domain-event support, `Money`, soft-delete, and common exception types, but those are coupled to its application conventions. Consumers need the useful primitives without taking that architecture.

## Goals / Non-Goals

**Goals:** provide a small net8.0 package with explicit APIs, immutable value semantics where practical, and no third-party dependencies; preserve application ownership of identifiers, persistence, and event dispatch.

**Non-Goals:** mediator integration, event bus dispatch, EF conventions, HTTP status decisions, or a replacement domain framework.

## Decisions

- Put primitives in `Platform.Domain`, depending only on `Platform.Core`. This avoids growing `Platform.Core` and makes adoption explicit.
- Use `IEntity<TId>`, `IAggregateRoot<TId>`, `IDomainEvent`, and `IHasDomainEvents` contracts plus small base implementations. Applications may implement the contracts without inheriting.
- Make domain events inspectable and clearable, but never dispatch them automatically. This keeps transaction and delivery policy application-owned.
- Implement `Money` with decimal amount and normalized ISO-like currency text, rejecting cross-currency arithmetic. Do not add exchange-rate behavior.
- Represent soft-delete and tenant scope as opt-in marker interfaces; do not add query filters.
- Keep domain exceptions carrying `Error` values so ASP.NET adapters can map them without a dependency from the domain package.

Alternatives considered: copying the starter `BaseEntity` and event implementation would import its conventions; adding everything to `Platform.Core` would make the foundational package grow and increase its compatibility surface; using a third-party value-object package would violate the small dependency boundary.

## Risks / Trade-offs

- [Risk] Consumers interpret base classes as mandatory persistence models → document interfaces as the preferred integration seam and test inheritance-free use.
- [Risk] Money semantics are mistaken for billing policy → limit the API to arithmetic and validation and state that rounding policy remains application-owned.
- [Risk] Event collection is used as a durable outbox → document that events are transient aggregate state and provide no persistence guarantees.

## Migration Plan

Install the package only in consumers that need the primitives. Existing types remain valid. Rollback is package removal and replacement with application-owned types; no schema or data migration is involved.

## Open Questions

None for the first implementation; identifier equality and event dispatch policy remain application-owned by design.
