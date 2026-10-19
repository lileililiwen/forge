## Why

The starter kit enforces tenant isolation by default and supports tenant-specific database connections, while the platform persistence package currently requires consumers to configure filters and scopes manually. That gap makes incremental adoption error-prone and leaves a high-risk class of data leaks to application code.

## What Changes

- Add an optional tenant-provider adapter for EF Core and ASP.NET Core.
- Add default-on tenant isolation, tenant scope propagation, and tenant-aware connection selection as explicit opt-in behaviors.
- Add safe hooks for tenant provisioning and migration readiness without owning tenant entities or migrations.
- Add tests for missing, global, shared-database, and dedicated-database tenant modes.

## Capabilities

### New Capabilities

- `platform-persistence-multitenancy`: Provider-neutral tenant scope plus optional EF/ASP.NET provider integration.

### Modified Capabilities

- None.

## Impact

Adds optional Finbuckle/EF Core integration packages and public tenant contracts. Existing explicit `ITenantScoped`/`ITenantScope` APIs remain compatible. Applications retain tenant records, resolution policy, connection strings, migrations, and provisioning workflows.
