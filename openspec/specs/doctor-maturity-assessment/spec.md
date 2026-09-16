# doctor-maturity-assessment Specification

## Purpose
TBD - created by archiving change doctor-maturity-assessment. Update Purpose after archive.
## Requirements
### Requirement: Doctor finding inventory

Forge SHALL implement forge doctor checking manifest, missing/outdated features, dependency drift, build/deployment configuration, repository, CI, documentation and maturity requirements, returning evidence and remediation classification.

#### Scenario: Doctor finding inventory success

- **WHEN** a fixture contains known missing configuration and drift
- **THEN** doctor reports stable finding IDs and the corresponding evidence and classifications

#### Scenario: Doctor finding inventory failure

- **WHEN** a required inspector cannot run
- **THEN** doctor reports the check as unavailable and does not label the project healthy

#### Scenario: Doctor finding inventory boundary

- **WHEN** doctor runs on a clean unchanged project twice
- **THEN** findings remain equivalent and project files and remotes remain unchanged

### Requirement: Evidence-based maturity levels

Forge SHALL assess L0 prototype, L1 structure/configuration/database where applicable/logging/health/build definition, L2 applicable auth/admin/CI/DriftWatch/deployment/audit, L3 identity compatibility/observability/release/registry/distribution/upgrades, and L4 applicable backup/recovery/monitoring/security/privacy/secrets/alerts against current and target levels.

#### Scenario: Evidence-based maturity levels success

- **WHEN** a project requests an L2 assessment
- **THEN** missing applicable L1 and L2 controls are reported with evidence

#### Scenario: Evidence-based maturity levels failure

- **WHEN** a manifest declares L4 but recovery evidence is missing
- **THEN** doctor reports the unmet control rather than granting production status

#### Scenario: Evidence-based maturity levels boundary

- **WHEN** an L0 prototype has neither deployment automation nor an applicable database requirement
- **THEN** doctor respects L0 and records nonapplicability rather than forcing infrastructure

