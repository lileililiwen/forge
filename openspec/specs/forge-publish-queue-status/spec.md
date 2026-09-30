# forge-publish-queue-status Specification

## Purpose
A durable, watchable `forge deploy status` over the per-stage journal, with sequential fleet execution and provider progress events.
## Requirements
### Requirement: Sequential fleet execution

The Forge fleet publisher SHALL invoke at most one project provider operation
at a time and SHALL not start the next eligible project until the current
project has a terminal provider response.

#### Scenario: Projects run in deterministic order

- **WHEN** a fleet contains projects `alpha` and `beta`
- **THEN** Forge SHALL invoke `alpha` first, wait for its terminal response,
  and invoke `beta` only afterward

#### Scenario: First project fails

- **WHEN** `alpha` returns a failed terminal response and `--fail-fast` is set
- **THEN** Forge SHALL persist `alpha` as failed and SHALL NOT invoke `beta`

### Requirement: Provider progress events

The provider contract SHALL support bounded, secret-free `publish.progress`
events that Forge can display while the provider operation is running.

#### Scenario: Mac build progress is visible

- **WHEN** the Jenkins provider enters preflight, transfer, build-and-run,
  verification, or routing
- **THEN** Forge SHALL display the project, phase, and status before the final
  response is returned

#### Scenario: Malformed progress does not fabricate success

- **WHEN** a provider emits malformed progress data and no valid terminal
  response
- **THEN** Forge SHALL report provider failure or timeout and SHALL NOT report
  the project as healthy

### Requirement: Durable deploy status

Forge SHALL persist project-level publish state in its existing operations
journal and SHALL provide `forge deploy status` for fleet-wide, queue-specific,
and project-specific queries.

#### Scenario: Query active fleet

- **WHEN** `forge deploy status` is run while a fleet is publishing
- **THEN** the output SHALL identify the queue, active project, current phase,
  completed projects, and failed projects without creating a new journal row

#### Scenario: Query completed fleet

- **WHEN** all projects have terminal states
- **THEN** status SHALL report the per-project terminal states and aggregate
  success/failure counts with a success exit only when all requested projects
  are healthy

#### Scenario: Unknown project

- **WHEN** `forge deploy status --project missing` is requested
- **THEN** Forge SHALL return a typed not-found result and a non-success exit
  without modifying the journal

### Requirement: Bounded status watch

`forge deploy status --watch` SHALL poll an existing queue at a bounded
interval and SHALL exit when the queue reaches a terminal aggregate or the
watch deadline expires.

#### Scenario: Watch reaches terminal state

- **WHEN** the active queue completes
- **THEN** watch SHALL print the final aggregate and exit successfully only if
  every requested project succeeded

#### Scenario: Watch deadline expires

- **WHEN** the queue remains active until the watch deadline
- **THEN** watch SHALL return the last observed state and a non-success exit;
  it SHALL not mark active projects as succeeded

