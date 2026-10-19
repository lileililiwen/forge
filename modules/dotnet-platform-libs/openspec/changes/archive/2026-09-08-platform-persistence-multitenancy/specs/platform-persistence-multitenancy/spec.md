## ADDED Requirements

### Requirement: Explicit tenant scope

The tenant integration SHALL expose a scoped current-tenant abstraction that can represent a tenant, an intentional global operation, or an unresolved request, and SHALL make tenant scope available before resolving tenant-filtered services.

#### Scenario: Background tenant operation
- **WHEN** a background handler begins with a tenant identifier
- **THEN** the tenant scope is installed before its DbContext and dependent services are resolved

### Requirement: Default tenant isolation

The EF Core adapter SHALL apply tenant isolation to opted-in tenant-scoped entities by default and SHALL require an explicit global marker for entities that intentionally cross tenants.

#### Scenario: Missing global marker
- **WHEN** a newly mapped tenant entity has no explicit global marker
- **THEN** queries and writes are constrained to the current tenant scope

### Requirement: Fail-closed tenant access

The adapter SHALL reject tenant-scoped database access when no tenant scope exists unless the application explicitly opens a global operation scope.

#### Scenario: Unresolved request
- **WHEN** a tenant-scoped query executes without a tenant scope
- **THEN** the adapter fails with a safe configuration or scope error instead of returning cross-tenant rows

### Requirement: Connection routing seam

The adapter SHALL allow an application-owned resolver to select a shared or dedicated connection and SHALL expose the selected connection through a scoped provider suitable for transaction sharing.

#### Scenario: Dedicated tenant database
- **WHEN** the resolver returns a dedicated connection for tenant A
- **THEN** all participating DbContexts in the scope use that connection provider and do not silently open an unrelated connection
