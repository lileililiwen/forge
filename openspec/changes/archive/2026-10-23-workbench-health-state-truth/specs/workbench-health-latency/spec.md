## MODIFIED Requirements

### Requirement: Fast workbench open

Forge SHALL open a project's workbench detail without running the
external policy checker or any unbounded inspection inline. The detail
GET's `health` projection SHALL be a local pass (manifest, profile,
features, git probes) completing on disk-cache timescales, and SHALL
append one explicit `unavailable` finding (`policy-deferred`) whenever
the live policy check did not run for the returned view. A locally-clean
project SHALL report `health.state: deferred` — never `healthy` — so no
row claims a pass it did not compute. Local problems SHALL still report
`issues`; `stale` and `unavailable` keep their exact meanings. The
projected state SHALL be derived from the doctor's own health partition:
a report that carries no blocking finding and no unmet control and whose
observation is stale SHALL report `stale`, never `issues`.

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

#### Scenario: A stale observation is not a set of issues

- **WHEN** the operator opens a project whose registry observation
  predates its current manifest and whose local checks carry no failing
  evidence
- **THEN** the detail responds with `health.state: stale`
