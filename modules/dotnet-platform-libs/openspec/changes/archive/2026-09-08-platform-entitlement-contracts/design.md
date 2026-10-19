## Dependencies

This change depends on `platform-core-contracts`.

## Contract Model

Define opaque identifiers for plan and feature, a normalized subscription snapshot with status and period boundaries, an entitlement snapshot with active features and limits, a feature-check result, and usage-meter interfaces. The contracts must be serializable and must not reference Stripe, EF Core, HTTP, or a specific user type.

Unknown or invalid provider states must not produce an active entitlement by default. The consuming adapter owns the mapping from provider state to normalized state.

## Idempotency

Define a provider-neutral processed-event contract and store interface. The contract identifies an event, provider, received time, and processing result. It does not define the persistence schema.

## Verification

Test state mapping helpers, inactive defaults, feature decisions, limit decisions, period boundaries, and duplicate-event semantics. Add examples showing both user-scoped and tenant-scoped subjects.
