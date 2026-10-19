# Changelog

All notable platform package changes are recorded here. Releases follow
Semantic Versioning and include the verified package version and migration
notes for breaking changes.

## Unreleased

- Added `Platform.Identity.Contracts`, `Platform.Identity.AspNetCore`, and
  `Platform.Identity.Testing` for the identity lifecycle contracts (refresh
  rotation, password recovery, two-factor, impersonation).
- Added `Platform.Tenant.Lifecycle.Contracts`, `Platform.Tenant.Lifecycle`,
  `Platform.Tenant.Lifecycle.AspNetCore`, and `Platform.Tenant.Lifecycle.Testing`
  for resumable tenant provisioning, migration, and seed orchestration.
- Added `Platform.Testing.AspNetCore` with the in-memory
  `PlatformTestWebApplicationFactory` and the `PlatformTestEnvironments.Testing`
  constant.
- Added `RecordingEventBus` and `TransientFailureInjector` to `Platform.Testing`.
- Added the `platform-consumer-adoption-conformance` consumer fixture:
  pinned-version policy, machine-readable `eng/package-manifest.json`,
  upgrade/rollback smoke test, and adoption documentation.
- Added package release governance, API baseline checks, and consumer-oriented
  quality gates.
