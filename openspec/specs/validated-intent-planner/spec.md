# validated-intent-planner Specification

## Purpose
TBD - created by archiving change validated-intent-planner. Update Purpose after archive.
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

Forge SHALL resolve validated Intent into pinned dependency-ordered assembly and validation steps, explain component choices and expose unresolved custom work.

#### Scenario: Reviewable deterministic plan success

- **WHEN** a compatible request has certified supported parts
- **THEN** the plan uses those parts and schedules doctor, tests and configured quality checks

#### Scenario: Reviewable deterministic plan failure

- **WHEN** the project or catalog changes after plan review
- **THEN** apply refuses the stale plan until revalidated

#### Scenario: Reviewable deterministic plan boundary

- **WHEN** a requirement has no deterministic component
- **THEN** the plan reports a bounded glue/business/spec gap instead of claiming complete assembly or rewriting mature infrastructure

