# Billing provider abstractions

## ADDED Requirements

### Requirement: Billing SHALL normalize provider behavior

The platform SHALL expose provider-neutral checkout, portal, subscription, provider-event,
entitlement, and usage contracts.

#### Scenario: Consumer changes provider

- **WHEN** a consumer switches the configured billing provider
- **THEN** application entitlement checks continue to use the normalized contract
- **AND** provider identifiers remain in consumer configuration

### Requirement: Provider events SHALL be idempotent and ordered safely

The orchestration layer SHALL deduplicate provider events and handle repeated or out-of-order
events without granting stale entitlements.

#### Scenario: Duplicate webhook

- **WHEN** the same provider event is delivered twice
- **THEN** the first delivery may project state
- **AND** the second delivery is recorded as already processed without a duplicate mutation

### Requirement: Entitlements SHALL be explainable

Feature checks SHALL return an explicit allow/deny result with reason, expiry, usage, and
provider-state information where available.

#### Scenario: Expired subscription

- **WHEN** a feature is checked after the entitlement expiry
- **THEN** the result denies access with the stable expiry reason
