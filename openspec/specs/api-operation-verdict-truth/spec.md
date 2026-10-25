# api-operation-verdict-truth Specification

## Purpose
One rule names what an API operation row records: the outcome the request earned. A request that only planned is a planned operation, a request that applied the work is a completed operation, a request the shared rule refused is a blocked operation and a run that did not finish is a failed operation.
## Requirements
### Requirement: An API operation row names what the operation did

Forge SHALL record in an API operation row the outcome the operation
earned. A request that only planned SHALL be journalled as a planned
operation and SHALL NOT be journalled as a completed operation, because
a plan applies nothing. A request that applied the work SHALL stay a
completed operation, a request the shared rule refused SHALL stay a
blocked operation, and a request that did not finish SHALL stay a failed
operation. The HTTP operation boundary SHALL read the one rule for the
state and the detail it records, the detail SHALL name the run, and no
API route SHALL write an operation state of its own.

#### Scenario: A planned deployment is not a completed operation

- **WHEN** an authorized deployment request is accepted as a dry run
- **THEN** the operation row records a planned operation and not a
  completed one

#### Scenario: A planned upgrade is not a completed operation

- **WHEN** an authorized upgrade request is accepted as a dry run
- **THEN** the operation row records a planned operation and not a
  completed one

#### Scenario: A refused plan stays blocked

- **WHEN** a deployment plan is computed for a revision the shared
  delivery rule blocks
- **THEN** the operation row records a blocked operation

#### Scenario: An applied request is still a completed operation

- **WHEN** an authorized feature request applies the work
- **THEN** the operation row records a completed operation

#### Scenario: A run that did not finish is not a completed operation

- **WHEN** a deployment apply reports a failed run
- **THEN** the operation row records a failed operation and not a
  completed one

#### Scenario: One rule, every API writer

- **WHEN** the API routes record their operation rows
- **THEN** all of them read the one rule for the state and the detail
  they record
