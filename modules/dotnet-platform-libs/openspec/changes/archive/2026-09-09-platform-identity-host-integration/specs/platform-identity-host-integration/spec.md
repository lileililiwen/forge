## ADDED Requirements

### Requirement: Current-user claim projection

The host adapter SHALL project configured subject, email, tenant, role, and permission claims into `CurrentUser` and SHALL return an anonymous value for unauthenticated requests.

#### Scenario: Authenticated request
- **WHEN** a request principal contains configured identity claims
- **THEN** the accessor returns a normalized current user with distinct roles and permissions

### Requirement: Permission authorization

The adapter SHALL provide authorization policy helpers that evaluate application-provided permission names and optionally emit normalized authorization audit events.

#### Scenario: Denied permission
- **WHEN** an authenticated subject lacks the required permission
- **THEN** authorization fails and the configured audit hook receives a denied decision without token contents

### Requirement: Replaceable identity stores

The integration SHALL allow applications to provide credential, session, verification, and identity stores without requiring platform-owned user entities or migrations.

#### Scenario: Application session store
- **WHEN** an application registers its session store
- **THEN** identity session operations use that store and preserve normalized platform outcomes

### Requirement: Security configuration validation

The adapter SHALL validate explicitly selected authentication configuration on startup and SHALL redact secrets from failures and diagnostics.

#### Scenario: Missing JWT signing configuration
- **WHEN** an application selects JWT authentication without a valid signing configuration
- **THEN** startup validation fails with a secret-free configuration error
