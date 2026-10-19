## Dependencies

This change depends on `platform-core-contracts` and `platform-entitlement-contracts`. ASP.NET host fixtures are optional and must not make the core test doubles depend on a web host.

## Components

Provide a fake clock with advance/set operations, an in-memory entitlement service, a recording usage meter, and builders for common snapshots. Test doubles must expose recorded calls or state for the consuming test to assert.

Do not embed xUnit assertions, a particular mocking framework, or real network/database clients.

## Verification

Test deterministic time, entitlement invalidation, usage recording/counting, and builder defaults. Verify that production projects do not acquire the test package through package dependencies.
