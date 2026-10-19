## 1. Contracts and package structure

- [x] 1.1 Define tenant scope, global operation, tenant metadata, and connection-resolution contracts without Finbuckle or EF Core in `Platform.Core`.
- [x] 1.2 Create the optional EF/ASP.NET tenant adapter package and central dependency entries.
- [x] 1.3 Use `BaseDbContext.cs`, `TenantIsolationExtensions.cs`, and `ScopedDbConnectionProvider.cs` from the starter kit as reference implementations; copy only the safety behavior and replace application types.

## 2. Tenant-aware EF implementation

- [x] 2.1 Implement scope installation and restoration for HTTP, background, and explicit global operations.
- [x] 2.2 Implement default tenant filters, global markers, unique-index handling, and model validation.
- [x] 2.3 Implement scoped connection resolution and transaction-sharing behavior for shared and dedicated databases.
- [x] 2.4 Add readiness hooks for tenant database connectivity without owning migrations or tenant catalog persistence.

## 3. Verification and documentation

- [x] 3.1 Add tests for tenant isolation, global entities, missing scope, scope restoration, and dedicated connections.
- [x] 3.2 Add architecture tests proving pure platform packages remain framework-neutral.
- [x] 3.3 Add an adoption guide mapping starter multitenancy files to the new adapter and documenting rollback.
- [x] 3.4 Run serial restore/build/test, `git diff --check`, and strict OpenSpec validation.
