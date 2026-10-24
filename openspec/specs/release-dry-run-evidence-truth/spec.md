# release-dry-run-evidence-truth Specification

## Purpose
A release dry run writes nothing and says so: one gate decides whether a release stage may write, every write-capable stage consults it, and the report of a dry run states that no side effect was attempted.
## Requirements
### Requirement: A release dry run writes nothing and says so

Forge SHALL hold one rule that decides whether a release stage may write
to the host, the repository or a provider: a stage writes only when the
release is confirmed and the run is not a dry run. Every write-capable
stage SHALL consult that rule before it writes, and a stage that may not
write SHALL record a status that is not a delivery together with a note
that names the reason it did not write. The report of a dry run SHALL
state that no side effect was attempted, and no stage outcome in that
report SHALL claim a delivery.

#### Scenario: A dry run writes nothing

- **WHEN** a release is applied as a dry run
- **THEN** the repository carries no new commit and no new tag, no provider is contacted, and every stage outcome records a status that is not a delivery

#### Scenario: The dry-run note and the stage outcomes agree

- **WHEN** the report of a dry run is rendered for a human or as JSON
- **THEN** the note says that no side effect was attempted, and no stage outcome in the same report claims a delivery

#### Scenario: A plan-only mirror stage is not a delivery

- **WHEN** a release is applied without a confirmation
- **THEN** the mirror stage reports the plan it collected and a status that is not a delivery

#### Scenario: A confirmed release still writes

- **WHEN** a release is applied with a confirmation and without a dry run
- **THEN** the write-capable stages run and write, exactly as before

#### Scenario: The journal records the plan, not an applied release

- **WHEN** a dry run of a release apply is recorded in the registry journal
- **THEN** the row records a planned run rather than a completed one
