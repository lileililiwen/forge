## Context

The starter kit validates boundaries through `src/Tests/Architecture.Tests`, integration behavior through Testcontainers, migration behavior through a DbMigrator smoke container, and distribution through template smoke tests.

Starter-kit references:

- `dotnet-starter-kit/src/Tests/Architecture.Tests/`
- `dotnet-starter-kit/src/Tests/Integration.Tests/`
- `dotnet-starter-kit/src/Tests/Integration.Middleware.Tests/`
- `dotnet-starter-kit/.github/workflows/backend.yml`
- `dotnet-starter-kit/.github/workflows/template-smoke.yml`

Agents may copy test structure and assertions, but the platform fixture must consume packed artifacts from a local package feed rather than a cross-repository `ProjectReference`.

## Goals / Non-Goals

**Goals:**

- Provide deterministic package-consumer fixtures for composition, replacement, health, failure, and boundary checks.
- Verify packages independently and in small compatible groups.
- Distinguish source/test failures from missing Docker, database, credentials, or network environment.

**Non-Goals:**

- Testing product modules, business billing rules, frontend behavior, or a single mandatory application architecture.
- Making production packages depend on `Platform.Testing` or the fixture host.
- Requiring every adapter's external provider in every CI job.

## Decisions

1. Create a test-only conformance package/fixture with application-owned fake stores and provider doubles.
2. Pack platform projects to a local feed and restore a separate consumer fixture against package versions; do not use cross-repo project references.
3. Split gates into always-on contract/architecture tests and explicitly labeled Docker/provider integration jobs.
4. Include replacement tests proving consumer registrations can override defaults and opt-in packages do not install unrelated providers.

Alternative rejected: copying the starter's whole solution would test application behavior rather than package compatibility and would create a second monolithic architecture.

## Risks / Trade-offs

- [Risk] Fixture becomes another starter kit → keep it minimal, provider-neutral, and contract-focused.
- [Risk] Package feed drift hides source regressions → build and pack the current checkout before fixture restore.
- [Risk] External integration tests are flaky → isolate them, record exact environment blockers, and retain deterministic fakes.

## Migration Plan

Introduce the fixture for existing packages, then add each future adapter's conformance cases. CI first runs pack/restore/contract tests, followed by optional Docker/provider jobs. Rollback removes only the fixture workflow; package APIs are unaffected.

## Open Questions

- Whether the fixture belongs in `samples/` or a dedicated `tests/Platform.ConsumerConformance` project; choose the layout that keeps test-only dependencies isolated.
