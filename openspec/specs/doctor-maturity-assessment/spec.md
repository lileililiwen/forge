# doctor-maturity-assessment Specification

## Purpose
Read-only maturity assessment through forge doctor: a stable finding inventory (PASS/WARN/FAIL/UNAVAILABLE) with evidence and remediation classes, target-gated L0–L4 applicability, registry-observation staleness, and a build configuration judged against the build definition the project's profile expects, never masking missing evidence as healthy and never reporting a check the doctor cannot run as a failure of the project.
## Requirements
### Requirement: Doctor finding inventory

Forge SHALL implement forge doctor checking manifest, missing/outdated features, dependency drift, build/deployment configuration, repository, CI, documentation, delivery state and maturity requirements, returning evidence and remediation classification. Forge SHALL judge a project's build configuration against the build definition its profile expects, and SHALL report a check it cannot evaluate as unavailable rather than as a failure of the project.

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

#### Scenario: The build configuration is judged per profile

- **WHEN** doctor runs on a project whose profile expects a build definition
- **THEN** the `build-config` finding is `PASS` with the evidence naming the file when that definition is present, and `FAIL` with the evidence naming the expected file when it is missing

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

### Requirement: DriftWatch workspace marker detection

Doctor SHALL recognize the DriftWatch configuration surface that sibling
projects actually use — `driftwatch.toml`, `gate.toml`, `.ai-gate/gate.yaml`
and the `.driftwatch/` state directory — alongside legacy names, and SHALL
report presence as evidence only, never as maturity by itself.

#### Scenario: Checker-configured project detected

- **WHEN** a project contains only `driftwatch.toml` with configured
  checkers
- **THEN** doctor reports DriftWatch as configured and names the file

#### Scenario: Gate-manifest project detected

- **WHEN** a project contains only `.ai-gate/gate.yaml`
- **THEN** doctor reports DriftWatch as configured and names the file

#### Scenario: No markers

- **WHEN** a project contains none of the recognized names
- **THEN** doctor reports no DriftWatch configuration as a distinct state
  from a configured-but-failing run

