# Platform.Persistence.Multitenancy

`Platform.Persistence.Multitenancy` is the optional multitenancy adapter for `Platform.Persistence.EfCore`. It exposes a scoped current-tenant abstraction, an HTTP middleware that installs tenant scopes, explicit global-operation scopes for background work, and a scoped connection provider that supports shared and dedicated tenant databases.

The adapter owns no tenant entities, no tenant catalog, no migrations, and no database credentials. Applications supply the resolver, the connection resolver, the tenant catalog, and the readiness probe.

## Registration

```csharp
services.AddPlatformPersistenceEfCore();
services.AddPlatformPersistenceMultitenancy(options =>
{
    options.MaxTenantIdLength = 128;
    options.TenantHeader = "X-Tenant-Id";
    options.EnableDefaultTenantIsolation = true;
    options.FailClosedOnMissingScope = true;
});

services.AddSingleton<ITenantResolver, HeaderTenantResolver>();
services.AddSingleton<ITenantConnectionResolver, ApplicationTenantConnectionResolver>();
services.AddSingleton<ITenantConnectionReadinessProbe, ApplicationTenantConnectionProbe>();
services.AddHealthChecks().AddPlatformTenantConnectionReadinessCheck(appTenants, appProbe);
```

The middleware is added to the request pipeline after the platform ASP.NET Core middleware but before tenant-scoped endpoints:

```csharp
app.UsePlatformAspNetCore();
app.UsePlatformMultitenancy();
app.MapPlatformEndpoints();
```

## Applying the default tenant filter

Applications call `modelBuilder.ApplyDefaultTenantFilters(scope)` from their `OnModelCreating`. The helper iterates over the model and applies the documented query filter to every entity that implements `ITenantScoped` and is not marked with `IGlobalTenantEntity`. The default behavior is to filter to the current resolved tenant; explicit `GlobalOperation` scopes remove the filter.

```csharp
protected override void OnModelCreating(ModelBuilder modelBuilder)
{
    modelBuilder.ApplyDefaultTenantFilters(_tenantAccessor.Current);
    base.OnModelCreating(modelBuilder);
}
```

For administrative paths that intentionally cross tenants the application calls `IgnoreQueryFilters()` per query. For background handlers and explicit cross-tenant operations, wrap the work in a `using (factory.BeginGlobalOperation("reason"))` block to relax the filter and avoid fail-closed behavior.

## Background handlers

Background handlers must install a tenant scope before resolving tenant-scoped services. The factory is registered as a singleton and is safe to inject into `IHostedService` implementations.

```csharp
public sealed class ReindexHandler(ITenantScopeFactory factory, AppDbContext context)
{
    public async Task HandleAsync(string tenantId, CancellationToken cancellationToken)
    {
        using var _ = factory.BeginTenant(new AppTenantInfo(tenantId));
        await context.ReindexAsync(cancellationToken);
    }
}
```

The factory snapshots the prior scope and restores it on dispose, so nested background work cooperates with the request scope without leaking state.

## Connection routing

`ScopedTenantConnectionProvider` returns the connection descriptor that the application-owned resolver selects for the current scope. Dedicated tenant databases share the descriptor through the scoped provider so participating `DbContext` instances can enroll in a single transaction; the shared connection is the default when the resolver does not select a dedicated database.

The provider fails closed by default when no scope is resolved. Set `MultitenancyOptions.FailClosedOnMissingScope = false` only for global operations that intentionally target the shared connection.

## Readiness

`TenantConnectionReadinessCheck` aggregates the outcomes of an application-owned `ITenantConnectionReadinessProbe` over the supplied tenant list and projects the result to a `ready` health check. Add it alongside the platform readiness endpoint to surface tenant-specific outages without leaking connection strings or credentials.

## Rollback

The adapter is opt-in. To roll back, remove `AddPlatformPersistenceMultitenancy`, the `UsePlatformMultitenancy` call, and the `modelBuilder.ApplyDefaultTenantFilters(scope)` line, then revert the application to the explicit `ITenantScope` filter. The pre-existing `Platform.Persistence.EfCore` `ITenantScope`/`ITenantScoped` contracts remain compatible and continue to compile.
