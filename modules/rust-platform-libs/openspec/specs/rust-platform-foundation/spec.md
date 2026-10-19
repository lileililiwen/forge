# rust-platform-foundation Specification

## Purpose

The Rust platform foundation provides small, opt-in crates that carry
language-neutral contract envelopes, identity / tenant / lifecycle
contracts, audit event types, and deterministic test support. Every
production crate is provider-neutral: it does not depend on Axum, SQLx,
any database driver, cookie/session implementations, OIDC/JWT providers,
or any product crate. The application owns persistence, session storage,
provider configuration, credential material, and migrations. The
foundation only exposes types, traits, and a JSON-Schema-driven
compatibility validator that the `platform-contracts` repository
certifies.
## Requirements
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

### Requirement: Operational contract families have typed payloads and parity walkers

The `platform-contracts` crate MUST expose typed, serializable Rust
payloads for `platform.readiness`, `platform.gate-result`,
`platform.job-outcome`, `platform.release-evidence`, and
`platform.capability`, each implementing `ContractPayload` with its
canonical family identifier, and the fixture walker MUST cover the full
12-case canonical matrix for each of the five families against the live
`platform-contracts` repository.

#### Scenario: Typed payload round-trip

- **WHEN** a minimal typed payload value for any of the five families
  is serialized to JSON and deserialized back
- **THEN** the round-trip yields the original value with all
  schema-required fields present

#### Scenario: Full-matrix parity

- **WHEN** the walker runs each family's manifest against the
  registry-backed validator
- **THEN** all 12 canonical cases per family (valid, invalid,
  boundary, unknown-field, redaction, compatibility) produce the
  expected valid/invalid outcome with the expected error and warning
  codes
