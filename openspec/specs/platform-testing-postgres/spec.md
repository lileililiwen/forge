# platform-testing-postgres Specification

## Purpose

Run PostgreSQL-backed integration tests against one Testcontainers container shared per test
assembly, migrating once into a template database and handing each test a freshly cloned
database, so a consumer stops paying a container boot plus a full EF migration replay per test
class and stops re-inventing the same fixture locally. The beneficiary is a consumer's
integration suite on a developer machine or CI runner that has Docker available. The boundary
is fixture lifecycle only: without Docker the gated tests report unavailable infrastructure and
skip rather than pass, provider fixtures stay out of the base testing package, and no product
schema, migration or data-access dependency is imported here.
## Requirements
### Requirement: One PostgreSQL container SHALL be shared per test assembly

The fixture SHALL boot a single container from a pinned (patch-level, never
`:latest` or untagged) PostgreSQL image per test assembly through the xUnit
`ICollectionFixture` pattern, expose the server connection string, and prove
it reached a real server with a version probe before use. Cleanup SHALL stop
and remove the container.

#### Scenario: Two test classes share one container

- **WHEN** two test classes in one run resolve the same collection fixture
- **THEN** both observe the same container id and container name.

#### Scenario: Floating image tag is rejected

- **WHEN** server options carry a `:latest` or untagged image
- **THEN** startup fails with a named diagnostic before touching Docker.

### Requirement: Migration SHALL run once into a template database

The caller SHALL supply migration as a
`Func<string, CancellationToken, Task>` receiving the template connection
string; the fixture SHALL invoke it exactly once per template database even
under concurrent seeding calls, with no EF or product-model dependency in the
package.

#### Scenario: Concurrent seeding runs migration once

- **WHEN** several callers seed the same template concurrently and then again
  sequentially
- **THEN** the migration delegate runs exactly one time.

### Requirement: Each test SHALL receive a fresh cloned database

Clones SHALL be created with `CREATE DATABASE ... TEMPLATE` from the seeded
template so each test observes the migrated schema without replaying
migrations. Caller-supplied names SHALL be preserved; the default name SHALL
be unique per call and every name SHALL be validated as a safe identifier.

#### Scenario: Row written in one clone is invisible in the next

- **WHEN** a test writes a row in one clone and a second clone is created
  from the same template
- **THEN** the second clone contains the migrated schema and the seed state
  but not the row.

### Requirement: Docker-gated tests SHALL skip without Docker

Integration tests requiring a container SHALL report unavailable Docker as
skipped with the reason rather than passing or failing on infrastructure.

#### Scenario: Docker unavailable

- **WHEN** the fixture cannot reach a Docker daemon
- **THEN** each Docker-gated test skips and does not claim fixture behavior
  was verified.

