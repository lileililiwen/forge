## MODIFIED Requirements

### Requirement: Doctor finding inventory

Forge SHALL implement forge doctor checking manifest, missing/outdated features, dependency drift, build/deployment configuration, repository, CI, documentation, delivery state and maturity requirements, returning evidence and remediation classification.

#### Scenario: Doctor finding inventory success

- **WHEN** a fixture contains known missing configuration and drift
- **THEN** doctor reports stable finding IDs and the corresponding evidence and classifications

#### Scenario: Doctor finding inventory failure

- **WHEN** a required inspector cannot run
- **THEN** doctor reports the check as unavailable and does not label the project healthy

#### Scenario: Doctor finding inventory boundary

- **WHEN** doctor runs on a clean unchanged project twice
- **THEN** findings remain equivalent and project files and remotes remain unchanged

#### Scenario: The delivery state is part of the inventory

- **WHEN** doctor runs for a registered project whose delivery state the delivery lane recorded
- **THEN** the report carries a `delivery-state` finding that names the delivery phase, and the finding is `FAIL` when the delivery failed, `WARN` while the delivery is in progress and `PASS` when it is healthy
