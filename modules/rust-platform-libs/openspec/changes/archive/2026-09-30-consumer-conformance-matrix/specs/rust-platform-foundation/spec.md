# rust-platform-foundation Specification

## ADDED Requirements

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
