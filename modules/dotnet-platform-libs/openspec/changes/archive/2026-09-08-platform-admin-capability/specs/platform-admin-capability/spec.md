# Administration capability

## ADDED Requirements

### Requirement: Administration endpoints SHALL be permission-protected

Every administrative operation SHALL require an explicit permission and SHALL produce a safe
forbidden response when the caller lacks it.

#### Scenario: Operator lacks user-management permission

- **WHEN** the operator requests the user-management endpoint
- **THEN** the endpoint rejects the request
- **AND** no user data is returned

### Requirement: Administrative queries SHALL be bounded

User, role, audit, session, and provider-status queries SHALL enforce maximum page sizes,
validated filters, and stable ordering.

#### Scenario: Oversized page request

- **WHEN** a caller requests more than the configured maximum page size
- **THEN** the endpoint returns a stable validation response
- **AND** it does not execute an unbounded query

### Requirement: Sensitive administration SHALL be audited

Role changes, account state changes, session revocation, and enabled impersonation flows SHALL
emit structured audit events without storing credentials or secrets.

#### Scenario: Impersonation enabled by an application

- **WHEN** an authorized operator starts impersonation with a reason
- **THEN** start and end events contain actor, target, reason, correlation, and expiry metadata
- **AND** the default configuration remains disabled
