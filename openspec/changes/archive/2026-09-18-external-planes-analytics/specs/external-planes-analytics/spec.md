## ADDED Requirements

### Requirement: External system identity and health

Forge SHALL integrate existing content and analytics providers through configured adapters retaining external identity, ownership and project mapping.

#### Scenario: External system identity and health success

- **WHEN** a project has a valid external content and analytics reference
- **THEN** inspect shows those references and adapter health

#### Scenario: External system identity and health failure

- **WHEN** external project mapping is missing or ambiguous
- **THEN** the adapter reports the mapping problem without attaching another project data

#### Scenario: External system identity and health boundary

- **WHEN** an integration is disabled
- **THEN** Forge continues local operations without contacting that provider

### Requirement: Timestamped project metrics

Forge SHALL aggregate project, agent, spec, quality, deployment and repository-star metrics with timestamps, source and explicit unavailable/stale states.

#### Scenario: Timestamped project metrics success

- **WHEN** all configured sources return observations
- **THEN** the summary reports counts and seven-day growth with the observation windows

#### Scenario: Timestamped project metrics failure

- **WHEN** a provider fails or returns an incompatible payload
- **THEN** its metrics are unavailable and the summary does not fabricate zeros

#### Scenario: Timestamped project metrics boundary

- **WHEN** observations span different time windows
- **THEN** the summary identifies those windows and avoids presenting them as one simultaneous measurement
