## Why

The platform will be consumed by 26+ repositories with different SDK versions, persistence models, and deployment environments. Current conformance coverage exists inside the platform repository, but consumers need a repeatable pinned-package adoption, upgrade, rollback, and architecture-verification route.

## What Changes

- Provide a consumer adoption template and checklist for private/local NuGet feeds.
- Expand packed-artifact conformance to cover registration replacement, health, failure safety, and opt-in boundaries.
- Add a pilot-consumer fixture using pinned package versions and no cross-repository project references.
- Document upgrade, rollback, vulnerability review, and production/test dependency rules.
- Add a machine-readable package capability/dependency manifest.

## Capabilities

### New Capabilities

- `platform-consumer-adoption`: repeatable package consumption and upgrade conformance.

### Modified Capabilities

- `platform-consumer-conformance`: extend conformance requirements to upgrades and rollback.

## Impact

Adds test fixtures, scripts, documentation, and package manifest data. It should not modify consumer repositories automatically.
