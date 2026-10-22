## MODIFIED Requirements

### Requirement: Honest sub-check states and safe reasons

Forge SHALL report each status sub-check (`doctor`, `check`, `readiness`) with
one of the states `healthy`, `issues`, `stale` or `unavailable`. A sub-check
that cannot run SHALL be reported `unavailable` with a safe, path-free reason
and SHALL never be reported as a pass. When any sub-check reports `issues` the
overall state SHALL be `issues`; otherwise when any sub-check is `unavailable`
the overall state SHALL be `unavailable`; otherwise when the `doctor` check is
`stale` the overall state SHALL be `stale`; otherwise the overall state SHALL
be `healthy`. The `readiness` sub-check SHALL be computed from the read-only
profile/readiness projection and SHALL NOT invoke the native readiness matrix,
generate fixtures, or run any build/test subprocess. The `check` sub-check
SHALL reuse the in-process checker assembly over the doctor report and the
read-only governance observation. The `doctor` sub-check SHALL judge with the
doctor's own blocking set — every finding that is `FAIL` or `UNAVAILABLE` —
so its state is the inverse of the doctor's own health field, and SHALL report
the failing findings, the unavailable findings and the unmet controls apart so
that a summary never reports no problems while its state reports issues.

#### Scenario: Healthy project

- **WHEN** every sub-check can run and reports no failing or missing evidence
- **THEN** the overall state is `healthy`

#### Scenario: Real finding reported honestly

- **WHEN** the in-process doctor or checker reports failing or warning evidence for the project
- **THEN** that sub-check and the overall state report `issues`, never `healthy` or a pass

#### Scenario: Sub-check cannot run

- **WHEN** a sub-check cannot read its inputs (for example the project directory is not readable)
- **THEN** it is reported `unavailable` with a safe reason that names no absolute filesystem path and no credential

#### Scenario: Readiness is the read-only projection

- **WHEN** the status route reports readiness for a project
- **THEN** it reports the profile's catalog support state only and performs no native build, test or fixture generation

#### Scenario: A check that could not run is named, not counted as a failure

- **WHEN** the doctor report carries an unavailable finding and no failing
  finding and no unmet control
- **THEN** the `doctor` sub-check reports `issues` and its summary names the
  unavailable count apart from the failing count
