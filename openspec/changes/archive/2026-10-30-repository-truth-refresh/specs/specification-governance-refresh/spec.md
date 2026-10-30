## ADDED Requirements

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
