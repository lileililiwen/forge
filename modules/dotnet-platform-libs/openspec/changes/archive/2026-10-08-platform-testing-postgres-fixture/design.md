# Design: platform-testing-postgres-fixture

## Package ownership and dependency direction

- New package `Platform.Persistence.Postgres.Testing` lives under the
  persistence family because it extends the Postgres testing story next to
  `Platform.Persistence.Postgres`, matching the per-component `*.Testing`
  convention (`Platform.Identity.Testing`, `Platform.Billing.Testing`, ...).
  The `.Testing` suffix keeps it inside the existing dynamic
  testing-only discovery rule, so production references fail the
  architecture suite without a new allow-list.
- Dependencies: `Npgsql` (raw SQL for `CREATE/DROP DATABASE`, backend
  termination, version probe) and `Testcontainers.PostgreSql` (container
  lifecycle) only. Zero `ProjectReference` entries, no
  `Microsoft.EntityFrameworkCore*`, no xUnit, no `TestServer`. This keeps the
  package usable by consumers regardless of ORM or test framework and honors
  the deferred-EF-helpers decision from `shared-test-web-factory` §10.
- Version `0.1.0` from the repo `VersionPrefix`; centrally pinned
  (`Directory.Packages.props`); packed like every other `Platform.*` package.

## API shape

```csharp
namespace Platform.Persistence.Postgres.Testing;

public sealed class PostgresTestServerOptions
{
    public const string DefaultImage = "postgres:17.6-alpine";
    public string Image { get; set; } = DefaultImage;      // patch-pinned; :latest or untagged rejected
    public string Username { get; set; } = "postgres";     // overridable
    public string Password { get; set; } = "postgres";     // overridable
    public string MaintenanceDatabase { get; set; } = "postgres";
}

public sealed class PostgresTestServer : IAsyncDisposable
{
    public static Task<PostgresTestServer> StartAsync(
        PostgresTestServerOptions? options = null,
        CancellationToken cancellationToken = default);
    public string ContainerId { get; }
    public string ContainerName { get; }
    public string ServerConnectionString { get; }
    public string BuildDatabaseConnectionString(string database);
    public Task EnsureTemplateSeededAsync(
        Func<string, CancellationToken, Task> migrateTemplateAsync,
        string templateDatabase = "platform_template",
        CancellationToken cancellationToken = default);
    public bool IsTemplateSeeded { get; }
    public Task<string> CreateIsolatedDatabaseAsync(
        string? databaseName = null,
        CancellationToken cancellationToken = default);
    public ValueTask DisposeAsync();
}
```

## Semantics

- **One container per assembly.** `StartAsync` boots a single
  `PostgreSqlContainer` from the pinned image and probes it with
  `SHOW server_version_num` over Npgsql before returning: a returned server
  has answered a real PostgreSQL server, never an in-memory fake. Consumers
  hold one instance in an xUnit `ICollectionFixture`, so every test class in
  the collection shares `ContainerId`/`ContainerName`. `DisposeAsync` stops
  and removes the container (Testcontainers `DisposeAsync` semantics).
- **Migrate once, clone per test.** `EnsureTemplateSeededAsync` creates the
  template database when missing, invokes the caller-supplied
  `migrateTemplateAsync` delegate exactly once per template name
  (semaphore-guarded, concurrent-safe), then records the seed. The delegate
  receives the template connection string and must dispose its connections
  before returning so later clones are not blocked. The package never
  references EF: the delegate body (EF `MigrateAsync`, raw SQL scripts, ...)
  is the caller's code.
- **Freshness.** `CreateIsolatedDatabaseAsync` terminates stray backends on
  the template (best effort), then runs
  `CREATE DATABASE "<clone>" TEMPLATE "<template>"`; without a seeded
  template it creates an empty database instead of failing. Caller names are
  validated as safe Postgres identifiers (`^[A-Za-z_][A-Za-z0-9_$]*$`,
  max 63 chars); the default is `test_<32 lowercase hex>`, unique per call.
  Each clone is a snapshot of the template at seed time: rows written in one
  clone are invisible in the next.
- **Docker gating.** The package itself throws the container-start failure
  through. Tests assert Docker availability up front and `Assert.Skip` with
  the reason when Docker is missing, per the testing-toolkit rule that
  unavailable infrastructure is skipped, never passed.

## Alternatives rejected

- Putting the fixture in `Platform.Testing`: rejected, the toolkit change
  explicitly isolates provider/Testcontainers fixtures from the base package.
- xUnit-coupled base fixture (`IAsyncLifetime` in the package): rejected, it
  would drag a test-framework dependency into a shipped package and break
  the no-test-framework architecture rule.
- EF `DbContext` factory parameter instead of a connection-string delegate:
  rejected, it would force a `Microsoft.EntityFrameworkCore` package
  dependency; a `Func<string, CancellationToken, Task>` keeps the contract
  ORM-neutral and matches "caller supplies migration".
