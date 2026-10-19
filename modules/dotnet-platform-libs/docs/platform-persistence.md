# Platform.Persistence

`Platform.Persistence.EfCore` is an optional, provider-neutral set of EF Core contracts and
conventions. Applications own their `DbContext`, entities, configurations, migrations, tenant
model, and business filters.

## Registration

```csharp
services.AddPlatformPersistenceEfCore(options =>
{
    options.EnableAuditInterception = true;
    options.EnableSoftDeleteInterception = true;
});

optionsBuilder.AddInterceptors(
    serviceProvider.GetRequiredService<PlatformSaveChangesInterceptor>());
```

Convention methods such as `ApplySoftDeleteFilter<TEntity>()` and
`ApplyTenantFilter<TEntity>(scope)` must be called for each application entity that should be
affected. No entity scanning or global filter is enabled by registration alone.

`PlatformSaveChangesInterceptor` uses `IClock` and `IActorAccessor`, both replaceable for tests
and host-specific identity. Soft deletion changes a tracked delete to an update and preserves the
row for `IgnoreQueryFilters()` and administrative workflows.

## Migrations and readiness

`EfCoreMigrationStatusReader` checks connectivity and pending migrations without calling
`Migrate` or changing the database. `EfCoreReadinessCheck` maps that status to a health check;
liveness remains the host's responsibility. Applications own when and how migrations are applied,
including rollback planning and deployment ordering.

## PostgreSQL adoption

Reference `Platform.Persistence.Postgres` only when an application selects Npgsql:

```csharp
optionsBuilder.UsePlatformPostgres(connectionString);
```

The adapter owns only Npgsql configuration. It does not create a context, run migrations, or add
application entities. Removing the adapter and its options call returns the host to provider-neutral
EF Core configuration.

## Multitenancy

Add `Platform.Persistence.Multitenancy` to install a scoped current-tenant abstraction, an
ASP.NET Core middleware that resolves the tenant per request, and the default-on
`ApplyDefaultTenantFilters` model customizer. The package owns no tenant entities, no catalog,
and no migrations; applications supply the `ITenantResolver`, `ITenantConnectionResolver`,
`ITenantConnectionReadinessProbe`, and the tenant list. See
`docs/platform-persistence-multitenancy.md` for the full adoption guide and rollback steps.
