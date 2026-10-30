# specification-governance-refresh Specification

## Purpose
Keep canonical OpenSpec and repository governance documents truthful, traceable and free of generated placeholders.
## Requirements
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

### Requirement: Repository counts are derived and checked

Forge SHALL verify every stated archived-change and canonical-spec count against
the directories that hold them. A document that restates a count SHALL fail the
governance check when the stated number differs from the computed number.

#### Scenario: Counts agree

- **WHEN** the governance check runs against the current archive and spec
  directories
- **THEN** every stated count equals the computed count and the check passes
  without mutating a file

#### Scenario: A document drifts

- **WHEN** a status document states an archived-change or canonical-spec count
  that differs from the directories
- **THEN** the check names the file, the line, the stated number and the
  computed number

#### Scenario: A directory changes

- **WHEN** a change is archived or a capability spec is promoted or removed
- **THEN** the computed counts move and any document that still states the old
  number fails until it is refreshed

#### Scenario: A document states no count

- **WHEN** a governance document describes the repository without a numeric
  count
- **THEN** the check does not require one and does not fail

