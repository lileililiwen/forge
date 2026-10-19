## ADDED Requirements

### Requirement: Identity lifecycle contracts SHALL be provider-neutral

The platform SHALL expose contracts for session, refresh-token, password-recovery, two-factor, and impersonation workflows using opaque identifiers and normalized outcomes, without referencing application Identity entities or schemas.

#### Scenario: Application supplies an Identity adapter
- **WHEN** an application registers an adapter for the lifecycle contracts
- **THEN** the platform can orchestrate the workflow without requiring the application's user entity or DbContext type

### Requirement: Refresh-token rotation SHALL be atomic

The refresh-token contract SHALL support one-time consumption and replacement such that concurrent requests cannot successfully reuse the same refresh token.

#### Scenario: Concurrent refresh
- **WHEN** two requests present the same valid refresh token
- **THEN** at most one request succeeds and the other receives a stable replay or revoked result

### Requirement: Lifecycle failures SHALL be safe

Lifecycle outcomes SHALL distinguish invalid, expired, revoked, reused, and policy-denied operations using stable codes while avoiding credential values, hashes, user-enumeration signals, and provider response details.

#### Scenario: Unknown password-recovery subject
- **WHEN** a recovery request targets an unknown subject
- **THEN** the public result does not reveal whether the subject exists

### Requirement: Impersonation SHALL fail closed

The platform SHALL require an application-provided authorization decision and SHALL provide audit context for impersonation start, end, expiry, and revocation.

#### Scenario: No impersonation policy is registered
- **WHEN** a caller requests impersonation without a policy implementation
- **THEN** the request is denied and no impersonation grant is created
