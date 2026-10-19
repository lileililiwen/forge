## Why

The platform exposes EF Core migration status and readiness helpers, but small consumers still need to write a separate migration executable. The starter kit demonstrates the operational value of keeping migrations outside API startup; that behavior should be available without adopting its tenant catalog, seed model, or host architecture.

## What Changes

- Add a reusable migration-runner host/CLI integration for application-owned `DbContext` types.
- Support pending-migration inspection, apply, optional application seed callback, cancellation, and safe diagnostics.
- Keep migration assemblies, connection strings, locks, tenant iteration, and seed data application-owned.

## Capabilities

### New Capabilities

- `efcore-migrator`: reusable application-owned EF Core migration execution boundary.

### Modified Capabilities

- None.

## Impact

New optional EF Core/tooling package and executable integration API. No existing database schema or runtime startup behavior changes.

## Non-Goals

- No platform migrations, tenant catalog, distributed lock implementation, or automatic API startup migration.
