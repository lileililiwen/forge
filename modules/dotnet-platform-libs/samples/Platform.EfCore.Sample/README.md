# EF Core sample (stage 2)

Application-owned persistence on top of the provider-neutral
`Platform.Persistence.EfCore` conventions.

## Owned by the application

- `SampleDbContext`, the `SampleItem` entity, and table mapping.
- The `Migrations/CreateSampleItems` migration fixture (hand-written
  here; generated migrations would live in the same folder).
- Provider selection (`Microsoft.EntityFrameworkCore.Sqlite`), the
  connection string, and the decision to call `MigrateAsync`.

## Owned by the platform

- Nothing about this schema. `AddPlatformPersistenceEfCore`
  registers only opt-in convention services (clocks, interceptors);
  all switches default off.

## Rollback

Remove the `Platform.Persistence.EfCore` reference and the
`AddPlatformPersistenceEfCore` call; the context, provider package,
and migrations keep working standalone.

## Non-goals

No tenant filters, soft-delete, or audit interception are enabled
here; the application opts into each convention explicitly when it
needs it.
