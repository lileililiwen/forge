# Proposal: platform-testing-postgres-fixture

## Why

Consumers run PostgreSQL-backed integration tests by booting one
Testcontainers container per test class and replaying full EF migrations per
test. That cost grows with the suite and each team re-invents the same fixture.
The `platform-testing-toolkit` change deferred optional Testcontainers fixtures
(tasks.md 3.2, unchecked) under the rule that provider fixtures stay out of the
base testing package and Docker-gated tests report unavailable infrastructure as
skipped rather than passing.

## What Changes

- Add one new test-only package, `Platform.Persistence.Postgres.Testing`:
  an assembly-shared PostgreSQL server fixture (one pinned-image container per
  test assembly via the xUnit `ICollectionFixture` pattern) plus
  template-database provisioning (migrate once into a template database, then
  `CREATE DATABASE ... TEMPLATE` per test in milliseconds).
- The package takes no EF dependency and owns no product model: the caller
  supplies migration through a delegate; per-test database names stay
  caller-chosen with a unique default.
- Add Docker-gated integration tests proving single-container sharing,
  clone freshness, and exactly-once migration; pure unit tests cover the
  image-pin guard and name validation without Docker.
- Update the architecture suite (project inventory, package-count tripwire,
  new dependency-boundary rules), the package manifest, and the docs.

## Capabilities

### New Capabilities

- `platform-testing-postgres`: assembly-shared PostgreSQL test fixture with
  template-database cloning for platform consumers.

### Modified Capabilities

- None.

## Impact

Adds one test-only `src/` package (`Npgsql` + `Testcontainers.PostgreSql`
dependencies) and Docker-gated tests. No existing public API changes; no
production package gains a reference.

## Non-goals

- No EF Core helpers (`ReplaceDbContext`, `EnsureCreatedAsync`,
  `ResetDatabaseAsync`), no `IAsyncLifetime`/xUnit coupling in the package.
- No product entities, migrations, connection strings, or provider defaults.
- No Redis/RabbitMQ/Hangfire fixtures; no change to `Platform.Testing` or
  `Platform.Testing.AspNetCore`.
- The fixture does not run migrations itself; it only hosts the server and
  the template-clone mechanism.

## Public API effect

New package surface only (`Platform.Persistence.Postgres.Testing` namespace).
Covered by this change; no existing baseline entry changes.
