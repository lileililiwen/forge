## MODIFIED Requirements

### Requirement: Reviewable deterministic plan

Forge SHALL resolve validated Intent into pinned dependency-ordered assembly and validation steps, explain component choices and expose unresolved custom work. A resolve SHALL record a planned run and SHALL NOT be recorded as a completed resolution, because it applies nothing. An applied step SHALL name the action it performed: a component the planner resolved is `resolved`, never a verified install.

#### Scenario: Reviewable deterministic plan success

- **WHEN** a compatible request has certified supported parts
- **THEN** the plan uses those parts and schedules doctor, tests and configured quality checks

#### Scenario: Reviewable deterministic plan failure

- **WHEN** the project or catalog changes after plan review
- **THEN** apply refuses the stale plan until revalidated

#### Scenario: Reviewable deterministic plan boundary

- **WHEN** a requirement has no deterministic component
- **THEN** the plan reports a bounded glue/business/spec gap instead of claiming complete assembly or rewriting mature infrastructure

#### Scenario: A resolved plan is not an applied plan

- **WHEN** an intent is resolved into a plan
- **THEN** the journal row records a planned run, the detail names the
  run, and no step is reported as applied

#### Scenario: A resolved component is not a verified install

- **WHEN** an apply step resolves a component in the catalog
- **THEN** the step's status names the resolve and never reports the
  component as verified or installed
