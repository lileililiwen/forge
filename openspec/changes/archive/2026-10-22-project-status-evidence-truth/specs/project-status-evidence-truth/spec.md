## ADDED Requirements

### Requirement: A status projection names the evidence behind its verdict

Forge SHALL judge a project's status sub-check with the doctor's own
blocking set — every finding that is `FAIL` or `UNAVAILABLE` — and SHALL
derive the sub-check's state from exactly that set, so the state is the
inverse of the doctor's own health field. Forge SHALL report the failing
findings, the unavailable findings and the unmet controls apart, so a
check that could not run is never counted as a failing check. A summary
SHALL NOT report no problems while its state reports issues.

#### Scenario: A finding that could not run

- **WHEN** the doctor report carries an unavailable finding and no failing
  finding and no unmet control
- **THEN** the doctor sub-check reports `issues` and its summary names the
  unavailable count

#### Scenario: A failing finding

- **WHEN** the doctor report carries a failing finding
- **THEN** the doctor sub-check reports `issues` and its summary names the
  failing count apart from the unavailable count

#### Scenario: A report with nothing wrong

- **WHEN** every finding passes or warns, no control is unmet and the
  observation is fresh
- **THEN** the doctor sub-check reports `healthy` and its summary reports
  no failing evidence

#### Scenario: A stale observation

- **WHEN** the doctor report's observation is stale and the report carries
  no blocking finding and no unmet control
- **THEN** the doctor sub-check reports `stale` and names the staleness

#### Scenario: A stale observation with an unavailable finding

- **WHEN** the doctor report's observation is stale and the report carries
  an unavailable finding
- **THEN** the doctor sub-check reports `issues` and names the staleness
  beside the unavailable count

### Requirement: The status projection and the doctor agree

Forge SHALL keep the project status projection and the doctor report
consistent: a report the doctor does not call healthy SHALL NOT be
projected as healthy, and the projection's partition SHALL be the inverse
of the doctor's own health field. A finding the checker reports as warning
evidence SHALL be counted by the doctor sub-check as well.

#### Scenario: The doctor is unhealthy and the projection is too

- **WHEN** the doctor report is not healthy
- **THEN** the doctor sub-check does not report `healthy`

#### Scenario: The sub-checks agree about the same evidence

- **WHEN** the doctor report carries unavailable findings
- **THEN** the doctor sub-check and the check sub-check both count them

### Requirement: The status evidence claims are pinned

Forge SHALL pin the status projection's state partition and its summary in
the repository's contract check, so that a summary cannot drift back into
naming a subset of the problems or no problem at all.

#### Scenario: The contract check reads the projection

- **WHEN** the repository's contract check reads the status projection
- **THEN** it fails when a report with an unavailable finding is projected
  as healthy or stale, or when a summary reports no problems while its
  state reports issues
