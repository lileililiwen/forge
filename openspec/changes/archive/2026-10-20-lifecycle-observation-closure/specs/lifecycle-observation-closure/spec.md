## ADDED Requirements

### Requirement: The doctor observes the delivery state

Forge SHALL report the delivery state the delivery lane recorded for the
project as a `delivery-state` finding in the doctor report, and SHALL judge
it with the delivery lane's own phase vocabulary: `PASS` when the delivery is
healthy, `FAIL` when the delivery failed, `WARN` while the delivery is in
progress, and not applicable when the project has no recorded delivery. The
finding SHALL name the delivery phase, the environment, the revision the
delivery transferred and the time the delivery state last changed.

#### Scenario: A healthy delivery is reported

- **WHEN** the doctor runs for a registered project whose delivery phase is `healthy`
- **THEN** the report carries a `delivery-state` finding with status `PASS`, naming the phase, the environment and the delivered revision

#### Scenario: A failed delivery is reported

- **WHEN** the doctor runs for a registered project whose delivery phase is `stage-failed` or `degraded`
- **THEN** the report carries a `delivery-state` finding with status `FAIL`, the project is not healthy, and the finding names the phase

#### Scenario: A delivery in progress is reported

- **WHEN** the doctor runs for a registered project whose delivery phase is `awaiting-stage-confirmation` or `awaiting-production-approval`
- **THEN** the report carries a `delivery-state` finding with status `WARN`, naming the phase, and the project stays healthy

#### Scenario: A project without a delivery is not applicable

- **WHEN** the doctor runs for a registered project that has no recorded delivery
- **THEN** the report records the `delivery-state` finding as not applicable, and does not warn about a delivery the project never declared

#### Scenario: A delivery state that cannot be read is visible

- **WHEN** the delivery state cannot be read
- **THEN** the report carries a `delivery-state` finding with status `UNAVAILABLE` that names the reason, and the project is not healthy

### Requirement: Observing a delivery writes nothing

Forge SHALL read the delivery state read-only: reading it SHALL NOT write a
journal row, change the registry bytes, write a target file, invoke a
provider or start a delivery verb. Every surface that reports a doctor
report — the CLI, the external checker and the API — SHALL read the delivery
state through the same function, so the finding is the same everywhere.

#### Scenario: The doctor writes nothing

- **WHEN** the doctor runs for a project whose delivery state it reads
- **THEN** the journal is unchanged, the registry bytes are unchanged, no target file is written and no delivery verb has run

#### Scenario: Every surface reads the same state

- **WHEN** the CLI, the external checker and the API doctor a project with the same delivery state
- **THEN** each report carries the same `delivery-state` finding with the same status and the same evidence
