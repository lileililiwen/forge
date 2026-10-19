## Why

The platform packages are independently adoptable, but there is no reusable consumer conformance suite proving that registrations, replacement seams, health behavior, isolation, and provider failures work together. The starter kit's architecture, integration, migration-smoke, and template-smoke tests expose the verification gap.

## What Changes

- Add a consumer conformance test toolkit and fixture host for platform packages.
- Test package registration, replacement, health/readiness, failure classification, tenant isolation, and opt-in boundaries.
- Add package-feed or local-NuGet verification rather than cross-repo project references.
- Add CI guidance for serial restore/build/test and environment-blocked classification.

## Capabilities

### New Capabilities

- `platform-consumer-conformance`: Reusable consumer fixture and verification gates for platform adoption.

### Modified Capabilities

- None.

## Impact

Adds test-only packages, sample fixture changes, CI/documentation updates, and optional local package-feed tooling. Production packages do not reference testing packages. No consumer application becomes a platform implementation dependency.
