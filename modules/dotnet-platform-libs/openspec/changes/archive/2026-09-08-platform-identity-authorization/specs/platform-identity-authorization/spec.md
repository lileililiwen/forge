# Identity and authorization capability

## ADDED Requirements

### Requirement: Current user SHALL be provider-neutral

The platform SHALL expose authenticated subject, email, tenant, role, and permission
information without requiring a particular token format or user entity.

#### Scenario: Anonymous request

- **WHEN** a request has no valid identity
- **THEN** the current-user contract reports an anonymous subject without throwing

### Requirement: Permissions SHALL be module-registered

Applications SHALL register resource/action permissions and use generated or named policies
without placing product-specific permissions in platform packages.

#### Scenario: Permission denied

- **WHEN** an authenticated user lacks a required permission
- **THEN** authorization returns the documented forbidden result
- **AND** the decision is available to the configured audit/diagnostic hook

### Requirement: External authentication SHALL be replaceable

OAuth/OIDC, password, email verification, and SMS providers SHALL implement independent
contracts with explicit failure classification and no secret leakage.

#### Scenario: Provider unavailable

- **WHEN** an external identity provider cannot be reached
- **THEN** the platform returns a provider-unavailable result
- **AND** it does not create or mutate a user identity
