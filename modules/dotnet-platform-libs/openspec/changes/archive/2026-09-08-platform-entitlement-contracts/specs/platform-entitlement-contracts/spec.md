## ADDED Requirements

### Requirement: Entitlements SHALL be provider-neutral

The entitlement contracts SHALL use opaque plan, feature, and subject identifiers and SHALL not expose provider SDK types or provider-specific price IDs.

#### Scenario: Stripe-backed application

- **WHEN** an application maps a Stripe subscription into the platform contract
- **THEN** the consuming adapter supplies opaque identifiers and the contract contains no Stripe type

### Requirement: Inactive access SHALL be the safe default

The contract SHALL represent anonymous, missing, expired, suspended, canceled, and unknown states as non-active unless an adapter explicitly produces an active state.

#### Scenario: Unknown provider status

- **WHEN** an adapter receives an unrecognized provider status
- **THEN** it produces a non-active entitlement and a diagnosable mapping result

### Requirement: Feature checks SHALL return structured decisions

Feature checks SHALL return whether access is allowed plus a stable reason and optional required plan or limit information.

#### Scenario: Paid feature for a free user

- **WHEN** a free user checks a paid feature
- **THEN** the result denies access with a stable reason that a web or non-web consumer can translate

### Requirement: Usage metering SHALL be replaceable

The platform SHALL expose usage-meter contracts without prescribing database schema, cache technology, or counting implementation.

#### Scenario: Rolling-window limit

- **WHEN** an application checks usage for a subject and metric over a window
- **THEN** the application-provided meter returns the count or decision without platform persistence assumptions

### Requirement: Processed events SHALL support idempotency

The webhook event contract SHALL allow an adapter to record and query processed provider events before applying state changes.

#### Scenario: Redelivered event

- **WHEN** the same provider event identifier is received twice
- **THEN** the second delivery can be acknowledged without applying the state transition twice
