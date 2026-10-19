## ADDED Requirements

### Requirement: Keep foundation crates provider-neutral

Foundation production crates MUST NOT depend on Axum, SQLx, database drivers,
cookie/session implementations, OIDC/JWT providers, or product crates.

#### Scenario: Dependency-direction check

- **WHEN** the architecture test inspects foundation dependencies
- **THEN** forbidden runtime/provider dependencies fail the check

### Requirement: Expose safe identity contracts

The identity crate MUST expose provider-neutral subject, session, permission,
tenant, and lifecycle outcomes without owning persistence or credential values.

#### Scenario: Application-owned store

- **WHEN** an application supplies its own identity store and provider
- **THEN** it can implement the platform traits without adopting a platform
  schema or database

#### Scenario: Safe failure

- **WHEN** authentication fails
- **THEN** the normalized outcome contains a stable failure category and no
  password, token, hash, or provider response detail

### Requirement: Match language-neutral contracts

Serialized Rust contract values MUST validate against the corresponding
language-neutral schema and compatibility fixtures.

#### Scenario: Valid fixture

- **WHEN** a Rust value is serialized for a supported contract version
- **THEN** the resulting document passes the canonical fixture validator

#### Scenario: Unsupported version

- **WHEN** a Rust consumer receives an unsupported major version
- **THEN** deserialization returns an explicit compatibility error
