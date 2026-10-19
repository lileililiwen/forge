# Proposal: EF Core persistence conventions

## Why

The C# products repeatedly implement EF Core registration, audit/soft-delete behavior,
pagination, specifications, migration checks, and database readiness. FullStackHero is a
useful reference, but its persistence building block is coupled to Finbuckle, SQL Server,
PostgreSQL, and application tenant types.

## Scope

Create optional `Platform.Persistence.EfCore` contracts and conventions, with a separate
PostgreSQL adapter only where necessary. Include reusable interceptors, query primitives,
migration/readiness helpers, and deterministic testing support.

## Non-goals

- no application `DbContext`, entity, migration, tenant class, invoice, or plan;
- no mandatory Finbuckle, PostgreSQL, SQL Server, or repository pattern;
- no automatic production migration on startup by default.

## API impact

Adds public EF Core abstractions, options, extensions, and test helpers. Existing platform
packages remain independent.
