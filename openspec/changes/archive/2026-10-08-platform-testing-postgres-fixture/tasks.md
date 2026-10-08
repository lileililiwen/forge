# Tasks: platform-testing-postgres-fixture

## 1. OpenSpec artifacts

- [x] 1.1 Author `proposal.md`, `design.md`, `tasks.md`, and
  `specs/platform-testing-postgres/spec.md` for this change.
- [x] 1.2 Validate with `openspec validate platform-testing-postgres-fixture --strict`.

## 2. Package implementation

- [x] 2.1 Add `src/Platform.Persistence.Postgres.Testing/` (`net10.0`,
  packable, test-only description; `Npgsql` + `Testcontainers.PostgreSql`
  package references only; centrally pinned versions).
- [x] 2.2 Implement `PostgresTestServerOptions` (pinned default image,
  `:latest`/untagged rejection, overridable username/password).
- [x] 2.3 Implement `PostgresTestServer` (start + real-server version probe,
  exactly-once template seeding, template clone with backend termination,
  identifier validation, dispose removes the container).
- [x] 2.4 Add the project to `Platform.sln`.

## 3. Tests

- [x] 3.1 Docker-gated integration tests in
  `tests/Platform.Persistence.Postgres.Tests`: single container shared across
  two test classes (container id/name equality), clone freshness (row in one
  clone invisible in the next), migration exactly once (spy delegate under
  concurrent + sequential seeding), caller-chosen and default unique names.
- [x] 3.2 Docker-free unit tests: image-pin guard, database-name validation,
  connection-string builder.
- [x] 3.3 Docker-unavailable reports `Skipped`, never `Passed`.

## 4. Repository wiring

- [x] 4.1 Pin `Npgsql` and `Testcontainers.PostgreSql` in
  `Directory.Packages.props`.
- [x] 4.2 Register the new project in `ProductionProjects`; add
  Postgres.Testing dependency-boundary rules; move the manifest-count
  tripwire 78 → 79.
- [x] 4.3 Regenerate `eng/package-manifest.json`
  (`scripts/generate-package-manifest.sh --check` passes).
- [x] 4.4 Document the package in `docs/packages.md` and the README Testing
  matrix.

## 5. Verification and archive

- [x] 5.1 `dotnet build` the new package + `dotnet test`
  `tests/Platform.Persistence.Postgres.Tests` green.
- [x] 5.2 `dotnet test tests/Platform.Architecture.Tests` green.
- [x] 5.3 `openspec validate platform-testing-postgres-fixture --strict`
  and post-archive `openspec validate --specs --strict`; `git diff --check`;
  `scripts/check-public-api.sh` exit 0.
- [x] 5.4 Archive the change; commit implementation then HANDOFF per the
  repo workflow; do not push.
