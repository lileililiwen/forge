# Platform.Domain

Framework-neutral domain building blocks. The platform owns the primitive
contracts and the safe-error taxonomy. The application owns identifiers,
persistence, event dispatch, exchange rates, rounding policy, and query
behavior.

## Packages

| Package | Purpose |
| --- | --- |
| `Platform.Domain` | Typed entity identity, aggregate roots with transient domain events, `Money`, opt-in soft-delete/tenant markers, and safe domain exceptions. Depends on `Platform.Core` only. No ASP.NET Core, EF Core, Mediator, validation, or provider references. |

## Contracts

`IEntity<TId>` carries the typed identifier. `IAggregateRoot<TId>` combines
`IEntity<TId>` with `IHasDomainEvents`. The interfaces are the preferred
integration seam: applications may implement them directly without
inheriting the base classes.

## Entities and aggregates

`Entity<TId>` and `AggregateRoot<TId>` are optional base implementations.
`AggregateRoot<TId>.AddDomainEvent` records events in insertion order;
`ClearDomainEvents` empties the collection. Recorded events are never
dispatched by the platform — the application drains them inside its own
transaction and delivery policy. Treat the collection as transient
aggregate state, not as a durable outbox.

`DomainEvent` is the optional base record (`EventId`, `OccurredOnUtc`,
optional `CorrelationId`/`TenantId`). `DomainEvent.Create(factory)`
supplies a fresh identifier and the current UTC timestamp. It deliberately
does not extend any mediator notification type, so consuming applications
stay decoupled from a dispatch framework.

## Money

`Money` is a sealed record with a decimal `Amount` and a normalized
`Currency` code (trimmed, uppercased invariant). `Zero(currency)` defaults
to `USD`. `Add`/`Subtract` (and the `+`/`-` operators) reject
cross-currency arithmetic with a deterministic
`InvalidOperationException` naming both currencies. `Multiply` (and `*`)
scales by a scalar and keeps the currency. There is no exchange-rate
conversion; rounding policy remains application-owned.

## Markers

`ISoftDeletable` (`IsDeleted`, `DeletedOnUtc`, `DeletedBy`) and
`IHasTenant` (`TenantId`) are opt-in markers. The platform exposes the
marker data only and applies no query filters, mappings, or delete
behavior.

## Safe errors

`DomainException` carries a stable `Platform.Core.Results.Error` and no
HTTP or provider-specific status code. `DomainValidationException` carries
`platform.validation`, `DomainNotFoundException` carries
`platform.not_found`, and `DomainConflictException` carries
`domain.conflict`. Web boundaries translate `Error` through their own
mapper (for example, the `Platform.AspNetCore` problem-details mapper);
unknown codes fail safe to a generic server error.

## Adoption

Install the package only in consumers that need the primitives. Existing
application types remain valid; prefer implementing the contracts over
inheriting the bases where a persistence model already exists.

## Migration from the starter domain types

1. Replace the starter `IEntity<TId>` / `BaseEntity<TId>` usage with
   `Platform.Domain.IEntity<TId>` / `Entity<TId>` (or implement the
   interface on the existing class).
2. Replace `AggregateRoot<TId>` and `IHasDomainEvents` the same way;
   drain `DomainEvents` explicitly where the starter published them via
   Mediator — the platform never dispatches.
3. Replace the starter `Money` with `Platform.Domain.Money`; remove any
   reliance on exchange-rate or rounding helpers (application-owned).
4. Replace starter `CustomException` hierarchies (which carry HTTP status
   codes) with `DomainException` carrying an `Error`; move status mapping
   to the web boundary.
5. Rollback: remove the package reference and restore the
   application-owned types. No schema or data migration is involved.

## Security

- `DomainException.Message` is always the safe `Error.Message`. Inner
  exceptions are never surfaced to callers; inspect them only in logs.
- Cross-currency failure messages name the two currency codes only —
  never amounts, identifiers, or provider details.
- Step and event identifiers are public data; do not embed secrets or
  connection strings in domain events.
