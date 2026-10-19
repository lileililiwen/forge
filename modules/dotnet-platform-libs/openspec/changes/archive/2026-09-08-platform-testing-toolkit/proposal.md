## Why

Shared contracts are only useful if each application can test them without real time, payment providers, or a shared database. Common test doubles will make adoption safer and reduce duplicated setup.

## What Changes

- Add deterministic clock and entitlement test doubles.
- Add builders for normalized subscription and entitlement snapshots.
- Add package guidance that keeps test dependencies out of production packages.

## Capabilities

### New Capabilities

- `platform-testing-toolkit`: Test-only helpers for platform contracts and ASP.NET integration.

### Modified Capabilities

## Impact

Adds a test-only package and test conventions. No production application receives a test dependency transitively.

## Context

The applications currently use different xUnit and assertion versions. The toolkit should minimize assumptions and avoid forcing a single assertion library.

## Goals / Non-Goals

**Goals:**

- Provide deterministic primitives.
- Keep fakes explicit and inspectable.
- Support tests that use xUnit, NUnit, or other frameworks without embedding assertions.

**Non-Goals:**

- Provide a full integration-test harness.
- Start Docker containers automatically.
- Mock Stripe or EF Core globally.

## Decisions

- Put test doubles in a separate package.
- Return values and recorded calls rather than assertion-specific helpers.
- Depend on platform contracts only.

## Risks / Trade-offs

- A test package can become a dumping ground; every helper must map to a public platform contract.
- Different test frameworks may limit convenience APIs; framework-neutral fakes are preferred.
