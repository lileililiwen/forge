## ADDED Requirements

### Requirement: Quota resources are application-defined
The platform SHALL represent quota resources and subjects as opaque identifiers and SHALL not define product plans, prices, or commercial entitlements.

#### Scenario: Application defines a new metered resource
- **WHEN** an application uses resource `embedding_tokens`
- **THEN** the quota contract accepts it without a platform package change

### Requirement: Quota decisions explain capacity
The platform SHALL return requested, consumed, reserved, limit, remaining, subject, resource, and window information for a quota check or reservation decision.

#### Scenario: Request exceeds remaining capacity
- **WHEN** the requested amount is greater than remaining capacity
- **THEN** the decision denies the reservation and reports the remaining amount

### Requirement: Reservations are idempotent
The platform SHALL use a stable operation identifier so repeating the same reservation, settlement, or release does not apply the operation twice.

#### Scenario: Worker retries a reservation
- **WHEN** the same operation identifier is submitted again
- **THEN** the store returns the original decision without increasing reserved usage again

### Requirement: Concurrent reservations are safe
The platform SHALL require store implementations to prevent two concurrent reservations from exceeding the same subject/resource/window limit.

#### Scenario: Two workers reserve the final unit
- **WHEN** two workers concurrently request the last available unit
- **THEN** at most one reservation is accepted

### Requirement: Lifecycle transitions are explicit
The platform SHALL support settlement and release of accepted reservations and SHALL expose invalid transitions as safe failures.

#### Scenario: A reservation is released
- **WHEN** an accepted reservation is released with its operation identifier
- **THEN** reserved capacity becomes available and a repeated release has no additional effect
