## Context

`Platform.Persistence.EfCore` already reads migration status but intentionally does not apply migrations. The starter kit's separate DbMigrator is useful operationally, while its tenant traversal and seed behavior are application-specific.

## Goals / Non-Goals

**Goals:** provide a library that an application console host can call with its own service provider, context factory, migration assembly, and seed callback; return stable exit-oriented outcomes and redact diagnostics.

**Non-Goals:** shipping an opinionated executable name, database provider, migration lock, tenant catalog, or seed implementation.

## Decisions

- Add `Platform.Persistence.EfCore.Migrator` as an optional package depending on existing EF Core persistence contracts.
- Define `IMigrationRunner`, `MigrationRunnerOptions`, `MigrationRunResult`, and an application callback interface for seeding.
- Support `ListPendingAsync` and `ApplyAsync`; application code owns argument parsing and process exit codes through a thin adapter.
- Resolve `DbContext` through an application-provided factory/delegate so the platform does not own connection configuration.
- Report exception type and stable category only; never include connection strings, SQL, or provider response bodies.
- Do not acquire a lock internally. Provide an optional application callback for exclusive execution so consumers can use PostgreSQL advisory locks, deployment locks, or no lock.

Alternatives considered: a ready-made executable would force host configuration and provider choices; automatic `Database.Migrate` in API startup violates safe deployment separation; embedding tenant orchestration would duplicate application lifecycle policy.

## Risks / Trade-offs

- [Risk] Consumers run migrations concurrently → expose the lock callback boundary and document that deployment must supply one where required.
- [Risk] Migration failures need actionable detail → return a stable category plus application-controlled logging, while keeping platform result data secret-free.
- [Risk] EF Core version mismatch → centralize the package version and test against the repository's supported version.

## Migration Plan

Add the runner to a new console project and move existing application migration calls behind it. Rollback returns to the application's previous runner; the package never changes schema automatically.

## Open Questions

None; tenant iteration and lock implementation remain explicit application callbacks.
