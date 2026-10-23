# deploy-status-phase-evidence Specification

## Purpose
The deploy status projection of a publish row's phase evidence: one shared
rule that names a value the journal row does not carry `unknown`, so every
surface that reads the row — the CLI's human renderer, the CLI's JSON document
and the API's JSON document — states what the journal proves and what it does
not, and no entry reads as a verified success by silence.
## Requirements
### Requirement: A status projection names the evidence it does not have

Forge SHALL project every deploy status entry through one shared
projection, and every entry SHALL expose its `revision`,
`build_status`, `run_status` and `container_identity` fields. A value
the journal row does not carry SHALL be projected as `unknown` and
SHALL NOT be omitted, so an entry never reads as a verified success by
silence. A value the row does carry SHALL be projected as that value.

#### Scenario: A row with no phase evidence

- **WHEN** the journal row carries no revision, build status, run status
  or container identity
- **THEN** the entry exposes all four as `unknown` in the JSON document
  and the human renderer prints `unknown` for each of them

#### Scenario: A row with phase evidence

- **WHEN** the journal row carries a revision, a build status, a run
  status and a container identity
- **THEN** the entry exposes those values unchanged, and the human
  renderer prints them

#### Scenario: One surface, one rule

- **WHEN** the CLI's human renderer, the CLI's JSON document and the
  API's JSON document read the same journal row
- **THEN** all three expose the same projected values for it
