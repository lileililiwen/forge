# external-planes-analytics Specification

## Purpose
External content and analytics plane over configured adapters (unified-content and github-analytics supported, notion/confluence/gitlab-analytics/codeberg-analytics planned): per-provider health observations, windowed metric aggregates that never sum across windows, and credential-safe evidence. The inspection report carries a verdict that names what the inspection did.
## Requirements
### Requirement: External system identity and health

Forge SHALL integrate existing content and analytics providers through configured adapters retaining external identity, ownership and project mapping. The inspection report SHALL carry a verdict that names what the inspection did, so an inspection that invoked no adapter is reported and journalled as a plan and never as a completed health check, and the report's availability SHALL be read from the observations the inspection actually made.

#### Scenario: External system identity and health success

- **WHEN** a project has a valid external content and analytics reference
- **THEN** inspect shows those references and adapter health

#### Scenario: External system identity and health failure

- **WHEN** external project mapping is missing or ambiguous
- **THEN** the adapter reports the mapping problem without attaching another project data

#### Scenario: External system identity and health boundary

- **WHEN** an integration is disabled
- **THEN** Forge continues local operations without contacting that provider

#### Scenario: A planned inspection is never a completed health check

- **WHEN** an analytics inspection is run as a dry run and no adapter is invoked
- **THEN** each observation's status is `planned`, the report's verdict names the inspection a plan, the report is not healthy, and the journal records a planned inspection

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
