# Design

`Platform.Persistence.EfCore` owns provider-neutral EF Core conventions: auditable entities,
soft-delete contracts, domain-event dispatch hooks, UTC handling, pagination, specifications,
connection validation, migration status, and readiness checks. `Platform.Persistence.Postgres`
owns Npgsql-specific health/migration behavior.

Consumers register their own `DbContext` and entity configurations. A convention opt-in must
never scan or mutate unrelated application entities without explicit registration.

The package should support shared-database tenancy only through an abstraction such as
`ITenantScope`; isolated-database routing remains an adapter/application concern.

Architecture tests must reject product projects, Stripe, provider-specific domain types, and
unapproved tenant dependencies from the framework-neutral package.
