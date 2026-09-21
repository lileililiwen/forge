# specification-governance-refresh Specification

## Purpose

Keep canonical OpenSpec and repository governance documents truthful,
traceable and free of generated placeholders.

## ADDED Requirements

### Requirement: Canonical capability purposes and evidence status

Forge SHALL maintain a non-placeholder purpose for every promoted capability
and SHALL distinguish implementation, contract, native, provider and release
evidence in repository status documents.

#### Scenario: Canonical specs are publishable

- **WHEN** the canonical spec set is checked
- **THEN** every capability has an accurate purpose and no `TBD` placeholder

#### Scenario: Evidence is partial

- **WHEN** source/tests pass but native or provider evidence is unavailable
- **THEN** the docs identify the missing evidence and do not claim release readiness

#### Scenario: Historical evidence is retained

- **WHEN** an archived handoff entry describes an earlier cycle
- **THEN** it remains identifiable as historical and is not rewritten as current proof

### Requirement: Deterministic governance traceability

Forge SHALL provide a deterministic check for canonical placeholders, broken
local links, stale `current_spec` pointers and contradictory completion claims.

#### Scenario: Governance check passes

- **WHEN** all active and canonical documents satisfy the repository rules
- **THEN** the check reports pass with no mutation

#### Scenario: Governance gap is found

- **WHEN** a placeholder, broken link or stale pointer exists
- **THEN** the check reports the file and actionable rule failure

#### Scenario: Runtime evidence is absent

- **WHEN** only OpenSpec validation or compilation evidence exists
- **THEN** the check preserves a non-runtime status and does not promote completion
