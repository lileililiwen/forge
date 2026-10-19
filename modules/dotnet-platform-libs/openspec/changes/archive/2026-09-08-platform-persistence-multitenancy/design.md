## Context

The starter's `BaseDbContext`, `TenantIsolationExtensions`, `PersistenceExtensions`, `ScopedDbConnectionProvider`, and `TenantProvisioningService` implement default-on filtering, soft delete, shared transaction connections, and dedicated tenant databases.

Starter-kit references:

- `dotnet-starter-kit/src/BuildingBlocks/Persistence/Context/BaseDbContext.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Persistence/TenantIsolationExtensions.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Persistence/ScopedDbConnectionProvider.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Persistence/PersistenceExtensions.cs`
- `dotnet-starter-kit/src/Modules/Multitenancy/`

Agents may copy algorithms and tests as a starting point, but must replace `AppTenantInfo`, Finbuckle application types, and starter-specific migration services with platform contracts and application callbacks.

## Goals / Non-Goals

**Goals:**

- Make tenant scope explicit and safe for HTTP, background, and EF Core operations.
- Provide an optional adapter that applies tenant filters consistently and supports shared or dedicated connections.
- Detect missing tenant scope for tenant-scoped operations instead of silently querying all tenants.

**Non-Goals:**

- Owning tenant entities, tenant identifiers, resolver order, provisioning, migration files, or database credentials.
- Making `Platform.Core` or pure contracts depend on Finbuckle or EF Core.
- Replacing application authorization with a tenant filter.

## Decisions

1. Keep `Platform.Persistence.EfCore` provider-neutral and add a separate tenant adapter package for Finbuckle/ASP.NET integration.
2. Use an application-supplied `ITenantCatalog`/connection resolver seam; the platform consumes resolved metadata but never stores tenant records.
3. Default tenant-scoped entity configuration to isolation when the adapter is enabled, with an explicit global marker for cross-tenant entities.
4. Install tenant scope before resolving tenant-filtered DbContexts and background handlers. Dedicated connections must be obtained through a scoped connection provider so transactions can be shared.

Alternatives rejected: requiring every consumer to hand-write filters duplicates the starter's safety-critical behavior; embedding Finbuckle in core violates independent adoption.

## Risks / Trade-offs

- [Risk] Default-on filtering can break global entities → require an explicit global marker and model inspection tests.
- [Risk] Missing tenant scope can cause operational failures → fail closed for tenant-scoped access and provide an explicit global operation scope.
- [Risk] Dedicated database routing complicates migrations → expose readiness and resolver seams while keeping migration orchestration application-owned.

## Migration Plan

Consumers first add the adapter in shadow/test environments, run model and cross-tenant isolation tests, then migrate one DbContext. Existing application filters remain until verified; rollback disables the adapter and restores prior registration.

## Open Questions

- Final Finbuckle adapter package name and whether connection routing belongs in the same package or a separate `Platform.Persistence.TenantDatabases` package.
