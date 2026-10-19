## Why

Small applications currently need to recreate basic domain types that the starter kit provides, or pull in a large application framework. A minimal, framework-neutral set of primitives would make platform adoption useful without importing product models or a modular-monolith architecture.

## What Changes

- Add an optional `Platform.Domain` package for entities, aggregates, domain events, money, and marker interfaces.
- Add safe, provider-neutral domain exception types or mappings that integrate with existing `Platform.Core` errors.
- Keep the package free of ASP.NET Core, EF Core, Mediator, validation libraries, and product policy.

## Capabilities

### New Capabilities

- `domain-primitives`: reusable framework-neutral domain building blocks.

### Modified Capabilities

- None.

## Impact

This introduces a new public package and APIs under `Platform.Domain`. Existing `Platform.Core` APIs remain compatible. Architecture tests, package inventory, package documentation, and the manifest require updates.

## Non-Goals

- No invoice, subscription, tenant, catalog, ticket, or application entity types.
- No EF Core mapping or persistence behavior.
- No forced migration of existing consumers.
