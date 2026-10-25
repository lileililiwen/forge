# validated-intent-planner Specification

## Purpose
Validated intent planning from natural-language requests: schema-validated Intent with required/forbidden capabilities and constraints, incompatibility refusal before generation, and deterministic pinned assembly plans with staleness re-checks and confirm-gated apply. A resolve records a planned run and an applied step names the action it performed.
## Requirements
### Requirement: Validated intent boundary

Forge SHALL turn natural-language requests into schema-validated Intent containing required/forbidden capabilities and constraints before any project mutation or command execution.

#### Scenario: Validated intent boundary success

- **WHEN** a public Rust application request excludes billing
- **THEN** the Intent retains the public constraint and billing prohibition

#### Scenario: Validated intent boundary failure

- **WHEN** a model response includes invalid actions or incompatible Flutter/server-postgres capabilities
- **THEN** validation rejects it before generation and explains a compatible client/backend boundary

#### Scenario: Validated intent boundary boundary

- **WHEN** a request is ambiguous about a required architectural choice
- **THEN** the unresolved choice is surfaced without silently selecting a conflicting plan

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
