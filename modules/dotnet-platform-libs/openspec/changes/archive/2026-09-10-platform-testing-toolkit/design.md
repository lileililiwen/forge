## Context

Existing `Platform.Testing` provides clocks and billing test doubles; `Platform.Quota.Testing`, notification testing, AI testing, and identity testing are separate support projects. The starter test suite demonstrates useful TestServer and Testcontainers patterns, but its application modules must not be imported into the platform.

## Goals / Non-Goals

**Goals:**

- Make common contract tests easy to write and deterministic.
- Provide reusable host setup without forcing a database, provider, or scheduler.
- Keep test package dependency direction explicit and auditable.

**Non-Goals:**

- Provide a universal application test framework.
- Include production code, migrations, provider credentials, or live-service assumptions.
- Hide integration-test prerequisites such as Docker.

## Decisions

- Keep a small `Platform.Testing` core for clock, context, fake-store, and failure-injection primitives.
- Use separate optional packages for ASP.NET Core/TestServer and Testcontainers to avoid unnecessary dependencies.
- Prefer scenario builders and recorded calls over mocking-framework abstractions.
- Provide contract-test base classes that consumers can run against their adapter implementations.
- Mark Docker-gated tests explicitly and report unavailable Docker as skipped/unverified.

## Risks / Trade-offs

- [Risk] Test helpers encode unstable implementation details → build only on public platform contracts.
- [Risk] A large test package increases restore cost → split by capability and keep dependencies minimal.
- [Risk] Fakes pass while real providers fail → pair fakes with packed consumer and opt-in provider integration tests.

## Migration Plan

1. Inventory existing testing projects and consolidate only duplicated public-contract helpers.
2. Add core deterministic fixtures and migrate one platform test project.
3. Add optional web/provider fixtures after the core API is stable.
4. Publish test packages independently and document compatibility with production packages.
