## Why

The platform already provides ambient tenant scope, EF isolation, connection routing, and readiness checks. The starter kit additionally implements tenant provisioning, retries, migrations, seeding, expiry notices, and lifecycle jobs. These workflows recur across applications but their entities, connection policies, migrations, and seed data must remain application-owned.

## What Changes

- Add provider-neutral tenant lifecycle and provisioning contracts.
- Model ordered steps, idempotency, retryable/permanent failures, status, cancellation, and operator retry.
- Add optional job integration over existing `Platform.Jobs` contracts.
- Add readiness/status reporting for provisioning and migration operations.
- Explicitly exclude tenant records, migrations, connection strings, billing plans, and seed data.

## Capabilities

### New Capabilities

- `platform-tenant-lifecycle`: application-owned tenant provisioning and migration orchestration seams.

### Modified Capabilities

- `platform-persistence-multitenancy`: add lifecycle integration points without changing isolation behavior.

## Impact

Adds contracts and optional orchestration packages over existing persistence and jobs boundaries. Applications will implement tenant catalog access and lifecycle steps.
