## ADDED Requirements

### Requirement: A health projection states the health the doctor found

Forge SHALL judge a project's health from the doctor's own report through
one shared partition, and SHALL derive the projected state from exactly
that partition, so the state is the inverse of the doctor's own health
field. A report that carries a failing or unavailable finding, or an
unmet maturity control, SHALL be projected as `issues`. A report that
carries no blocking evidence and whose observation is stale SHALL be
projected as `stale`. A report that carries no blocking evidence and is
current SHALL be projected as the surface's own healthy state.

#### Scenario: A blocking finding

- **WHEN** the doctor report carries a failing or unavailable finding, or
  an unmet maturity control
- **THEN** the health document reports `issues`

#### Scenario: A stale observation

- **WHEN** the doctor report carries no blocking finding and no unmet
  control and its observation is stale
- **THEN** the health document reports `stale` and not `issues`

#### Scenario: A current report with no blocking evidence

- **WHEN** the doctor report carries no blocking finding, no unmet control
  and is not stale
- **THEN** the health document reports the surface's own healthy state

#### Scenario: A report the doctor does not call healthy

- **WHEN** the doctor report is not healthy
- **THEN** the health document does not report `healthy`

### Requirement: The two health projections agree

Forge SHALL keep the workbench health document and the project-status
doctor sub-check consistent: for the same doctor report they SHALL
reach the same partition, and a report the doctor does not call healthy
SHALL NOT be projected as healthy by either of them.

#### Scenario: The same report, two surfaces

- **WHEN** the workbench health document and the project-status route
  read the same doctor report
- **THEN** both carry the same partition for it

### Requirement: The health partition is pinned

Forge SHALL pin the health partition and the projected states in the
repository's contract check, so that the projected state cannot drift
back into reporting `issues` for a report that carries no blocking
evidence.

#### Scenario: The contract check reads the projections

- **WHEN** the repository's contract check reads the workbench detail
  and the explicit refresh for a stale project, a project with a failing
  local check, a locally-clean project and an unreadable directory
- **THEN** it fails when a projected state does not match the partition
