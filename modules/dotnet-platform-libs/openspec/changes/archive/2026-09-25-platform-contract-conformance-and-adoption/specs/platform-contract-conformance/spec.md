## ADDED Requirements

### Requirement: Preserve application ownership

Platform identity and admin packages MUST NOT own consumer users, roles,
credentials, migrations, tenant policies, or business dashboard data.

#### Scenario: Consumer-owned identity store

- **WHEN** a product adopts the platform identity lifecycle
- **THEN** its existing identity store and authentication scheme remain the
  source of truth

### Requirement: Validate shared contract compatibility

The .NET platform MUST provide deterministic conformance fixtures for shared
identity, permission, tenant, audit, Gate, and release evidence contracts.

#### Scenario: Compatible envelope

- **WHEN** a platform adapter emits a supported contract document
- **THEN** the conformance fixture accepts it

#### Scenario: Incompatible envelope

- **WHEN** a required field or status vocabulary is incompatible
- **THEN** conformance fails with the contract and field identified

### Requirement: Report adoption without fabricating completion

Adoption diagnostics MUST distinguish absent, configured, incompatible,
unverified, and verified package usage.

#### Scenario: Package present without runtime evidence

- **WHEN** a product references a package but has no native evidence
- **THEN** diagnostics report configured or unverified, never production-ready
