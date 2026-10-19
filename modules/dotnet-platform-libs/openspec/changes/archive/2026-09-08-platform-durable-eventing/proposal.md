## Why

`Platform.Eventing` currently provides an in-process bus, but application events are lost on process failure and handler failures have no durable retry or operator-visible state. Multiple sibling applications already implement outbox, inbox, leasing, and dispatch logic, so this is the highest-value reliability capability to extract next.

## What Changes

- Add framework-neutral eventing contracts for durable outbox and inbox processing.
- Add an optional EF Core persistence package for outbox/inbox records and dispatch services.
- Support leasing, retry scheduling, dead-letter state, duplicate suppression, and correlation/tenant metadata.
- Preserve the existing in-process bus as a development and test transport.
- Add architecture tests proving contract packages remain free of EF Core and transport dependencies.
- Add documentation and starter adoption guidance.

## Capabilities

### New Capabilities

- `durable-eventing`: durable outbox/inbox contracts, persistence, dispatch, retries, and observability seams.

### Modified Capabilities

- None.

## Impact

- New `Platform.Eventing.Contracts` and `Platform.Eventing.EfCore` packages, with a possible transport adapter later.
- Public APIs for outbox writing, inbox claiming, dispatch results, and event processing options.
- EF Core and relational database dependencies remain confined to the optional adapter.
- No application DbContext, migrations, event types, transport credentials, or business workflows are introduced.
