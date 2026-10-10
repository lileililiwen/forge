# workbench-health-latency Specification

## Purpose
The workbench detail GET stays fast by running only the local doctor pass inline and reporting `deferred` with a `policy-deferred` finding when the live policy check has not run; the full DriftWatch + doctor pass runs on demand behind `POST /v1/admin/projects/{id}/health/refresh`, and the health card renders the deferred state with an explicit full-check control instead of a remediate shortcut.
## Requirements
### Requirement: Fast workbench open

Forge SHALL open a project's workbench detail without running the
external policy checker or any unbounded inspection inline. The detail
GET's `health` projection SHALL be a local pass (manifest, profile,
features, git probes) completing on disk-cache timescales, and SHALL
append one explicit `unavailable` finding (`policy-deferred`) whenever
the live policy check did not run for the returned view. A locally-clean
project SHALL report `health.state: deferred` — never `healthy` — so no
row claims a pass it did not compute. Local problems SHALL still report
`issues`; `stale` and `unavailable` keep their exact meanings.

#### Scenario: Opening a project does not invoke the checker

- **WHEN** the operator opens a project whose directory is readable and
  whose local checks pass
- **THEN** the detail responds with `health.state: deferred` and a
  `policy-deferred` finding, without executing the configured checker
  binary

#### Scenario: Local problems still surface immediately

- **WHEN** the operator opens a project with failing local checks
- **THEN** the detail responds with `health.state: issues` naming the
  local failures, as today

### Requirement: Explicit full health check

Forge SHALL provide `POST /v1/admin/projects/{id}/health/refresh`
running the same live DriftWatch + doctor pass the detail GET runs
today and returning the full health document (`healthy`/`issues`/
`stale`, complete findings, no deferred finding). The route SHALL be
read-only in effect: no confirm/digest binding and no journal row. An
unreachable checker SHALL yield the full document with an `unavailable`
policy finding naming the cause — the same behavior the GET has today,
now on demand.

#### Scenario: Operator runs the full check

- **WHEN** the operator posts a refresh for a readable project
- **THEN** the configured checker binary executes and the response
  carries the complete live health document

#### Scenario: Checker missing at refresh time

- **WHEN** the operator posts a refresh with no checker binary available
- **THEN** the response is 200 with an `unavailable` policy finding
  naming the cause, and no invented pass

### Requirement: Honest health card

The workbench health card SHALL render the `deferred` state with its
reason and a **Run full health check** control showing progress while
the live pass runs, then re-rendering the card. The `policy-deferred`
row SHALL offer the refresh control and SHALL NOT offer a remediate
plan shortcut. All other states and rows render exactly as before.

#### Scenario: Deferred card offers the full check

- **WHEN** a project opens with deferred health
- **THEN** the card names the unrun policy check and the refresh
  control completes the live pass with visible progress

#### Scenario: Refresh failure keeps prior state

- **WHEN** the refresh request fails
- **THEN** the card shows an honest error and keeps the prior health
  rendering

