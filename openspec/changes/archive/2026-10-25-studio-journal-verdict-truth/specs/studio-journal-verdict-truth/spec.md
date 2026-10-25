## ADDED Requirements

### Requirement: A Studio journal row names the action the Core performed

Forge SHALL record in a Studio journal row the action the Studio Core
performed. A recorded refinement request SHALL be journalled as a
recorded request and SHALL NOT be journalled as a completed action,
because no refinement runs on that path. A saved spec SHALL stay a
completed save, and a preview row SHALL name the observed preview
state. Every Studio writer SHALL read the one rule for the state its
row records.

#### Scenario: A recorded refinement is not a performed refinement

- **WHEN** a refinement request is accepted, validated and recorded
- **THEN** the `studio.refine` row records the recorded request and not
  a completed action

#### Scenario: A saved spec is still a completed save

- **WHEN** the Core saves a validated AppSpec
- **THEN** the `studio.spec.save` row records the completed save

#### Scenario: A preview row names the observed state

- **WHEN** the preview becomes ready, fails to start or is stopped
- **THEN** the row records the observed preview state

#### Scenario: One rule, every Studio writer

- **WHEN** the spec-save writer, the refinement writer and the preview
  writers record a row
- **THEN** all read the one rule for the state the row records
