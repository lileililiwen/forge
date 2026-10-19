# Platform.Persistence.EfCore.Migrator

Application-owned EF Core migration execution boundary. The platform
owns inspection, apply, seed/lock seams, failure classification, and
the console exit-code adapter. The application owns the context
factory, migration assembly, connection strings, locks, tenant
iteration, and seed data.

## Packages

| Package | Purpose |
| --- | --- |
| `Platform.Persistence.EfCore.Migrator` | Optional runner: `IMigrationRunner` / `MigrationRunner`, `MigrationRunnerRequest`, `MigrationPendingResult`, `MigrationRunResult`, `MigrationFailure`, `IMigrationSeeder`, `IMigrationExclusiveExecutor`, `MigrationCommand`, `MigrationConsoleRunner`. Depends on `Platform.Persistence.EfCore` plus provider-neutral EF Core (`Microsoft.EntityFrameworkCore`, `Microsoft.EntityFrameworkCore.Relational`). No provider packages, no ASP.NET Core, no messaging or job packages. |

## Contracts

- `MigrationRunnerRequest` — factory `CreateContext` plus `SeedAfterApply`, optional `Seeder`, optional `ExclusiveExecutor`. Every operational choice arrives here; the platform creates nothing.
- `IMigrationRunner.ListPendingAsync` — returns pending identifiers in apply order without modifying schema or data. A database that does not exist yet reports every defined migration as pending.
- `IMigrationRunner.ApplyAsync` — captures pending, calls `MigrateAsync` when non-empty, then runs the seed callback when `SeedAfterApply` is set and a seeder is supplied. Seed runs after every successful apply, including a no-op one, so application seeds must be idempotent.
- `IMigrationSeeder.SeedAsync(context, ct)` — application seed against the migrated context.
- `IMigrationExclusiveExecutor.ExecuteExclusiveAsync(operation, ct)` — wraps a whole operation. Back it with PostgreSQL advisory locks, a deployment lock, or a test double. Absent means no lock is taken.
- Cancellation propagates `OperationCanceledException`; it is never converted into a failure result.

## Failure results

`MigrationFailure` carries a stable `MigrationFailureCategory`
(`Unavailable`, `MigrationFailed`, `SeedFailed`) and a fixed diagnostic
template naming the category and the exception type only. Results never
contain connection strings, SQL, exception messages, or provider
response bodies; a throwing context factory fails fast with an
`InvalidOperationException` whose message names the exception type only.

## Console adapter

`MigrationConsoleRunner.RunAsync(args, configure, ...)` parses
`apply` (default) / `list-pending`, `--seed`, and `-h|--help`, builds
the request through the application-supplied `configure` factory, and
returns exit codes: 0 success or help, 1 migration/seed/configuration/
cancellation failure, 2 unknown verb. Only secret-free categories and
diagnostics reach the console writers (injectable for tests).

## Deployment, locks, and rollback

- Run the migrator as a deployment step with an elevated-DDL connection
  string, never at API startup. API hosts keep `MigrateAsync` out of
  their startup path.
- Supply `IMigrationExclusiveExecutor` wherever concurrent runs are
  possible (multiple replicas, parallel CI jobs). The platform takes no
  lock on its own; concurrent lock-free applies against one database can
  interleave migration history rows.
- Rollback returns to the application's previous runner; the package
  never changes schema automatically and ships no down-migration policy.
  Keep application down-migrations (or backups) under the same
  ownership as the up-migrations.
- Tenant iteration stays application-owned: call the runner once per
  tenant context from the application loop.

## Security

- Failure results and console output are secret-free by construction;
  connection strings and SQL never cross the runner boundary.
- Seed and lock callbacks execute with the migrator's privileges — keep
  the elevated-DDL credential out of API hosts and logs.
- The pending-migration list is deployment metadata; it names
  migration identifiers only.
