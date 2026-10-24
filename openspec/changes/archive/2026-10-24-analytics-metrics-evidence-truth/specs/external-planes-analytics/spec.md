## MODIFIED Requirements

### Requirement: Timestamped project metrics

Forge SHALL aggregate project, agent, spec, quality, deployment and repository-star metrics with timestamps, source and explicit unavailable/stale states. A metric SHALL be reported with a value, a source and an observation timestamp only when Forge observed it; a metric whose source was not gathered SHALL be reported as `unavailable` with a note naming the missing evidence, and the summary SHALL NOT fabricate zeros. The summary's completion flag SHALL be true only when every metric it carries was measured.

#### Scenario: Timestamped project metrics success

- **WHEN** all configured sources return observations
- **THEN** the summary reports counts and seven-day growth with the observation windows

#### Scenario: Timestamped project metrics failure

- **WHEN** a provider fails or returns an incompatible payload
- **THEN** its metrics are unavailable and the summary does not fabricate zeros

#### Scenario: Timestamped project metrics boundary

- **WHEN** observations span different time windows
- **THEN** the summary identifies those windows and avoids presenting them as one simultaneous measurement

#### Scenario: A metric nobody measured is not a zero

- **WHEN** a metrics report is aggregated and a metric's source was not gathered
- **THEN** that metric is `unavailable` with a note naming the missing evidence, it carries no value and no observation timestamp, and the summary is not complete
