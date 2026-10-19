## Why

The platform has focused billing and quota test helpers, while the starter kit has broad unit, architecture, TestServer, and Testcontainers coverage. Shared consumers need deterministic test support for tenant scope, eventing, cache/storage/quota behavior, and web hosts without bringing test dependencies into production packages.

## What Changes

- Add optional test-support packages for common platform contracts and integration fixtures.
- Add deterministic fake stores, clocks, tenant scopes, failure injectors, and web host builders.
- Add provider-neutral scenario builders for eventing, caching, storage, quota, and identity lifecycle.
- Keep Testcontainers and provider-specific fixtures isolated from the base testing package.
- Enforce that production projects cannot reference any testing-support package.

## Capabilities

### New Capabilities

- `platform-testing-toolkit`: reusable deterministic test fixtures for platform consumers.

### Modified Capabilities

- None.

## Impact

Adds test-only projects and dependencies, potentially including optional Testcontainers packages. Production package dependency graphs and runtime APIs remain unchanged.
