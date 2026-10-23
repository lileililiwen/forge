## MODIFIED Requirements

### Requirement: Delegated policy execution

Forge SHALL invoke configured DriftWatch checks and map findings to project observations while retaining category, policy ID, severity, tool version and evidence. Every consumer of a policy report SHALL reach the same verdict about that report: a report whose applicable rule failed is a failure, a report produced against an older source state is stale, a report in which no rule applied is unavailable and never a pass, and a report in which every applicable rule passed is the only report that may be called healthy.

#### Scenario: Delegated policy execution success

- **WHEN** a supported DriftWatch instance returns findings
- **THEN** doctor displays normalized results linked to their original evidence

#### Scenario: Delegated policy execution failure

- **WHEN** DriftWatch is missing, incompatible, times out or emits invalid output
- **THEN** the run reports unavailable or failed without a quality PASS

#### Scenario: Delegated policy execution boundary

- **WHEN** a policy is not applicable to a profile
- **THEN** the reason is preserved rather than manufacturing a detector result

#### Scenario: A second consumer of the same report

- **WHEN** a surface other than the doctor reads the same policy report
- **THEN** it reaches the same verdict as the doctor and never captures
  the report as a healthy pass when the doctor does not call it healthy
