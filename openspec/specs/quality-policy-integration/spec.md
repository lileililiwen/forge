# quality-policy-integration Specification

## Purpose
Delegated quality execution through the DriftWatch policy plane: configured checks invoked with bounded waits, findings mapped to project observations with category, policy ID, severity, tool version and evidence, and unavailable (never pass) on adapter failure.
## Requirements
### Requirement: Delegated policy execution

Forge SHALL invoke configured DriftWatch checks and map findings to project observations while retaining category, policy ID, severity, tool version and evidence.

#### Scenario: Delegated policy execution success

- **WHEN** a supported DriftWatch instance returns findings
- **THEN** doctor displays normalized results linked to their original evidence

#### Scenario: Delegated policy execution failure

- **WHEN** DriftWatch is missing, incompatible, times out or emits invalid output
- **THEN** the run reports unavailable or failed without a quality PASS

#### Scenario: Delegated policy execution boundary

- **WHEN** a policy is not applicable to a profile
- **THEN** the reason is preserved rather than manufacturing a detector result

### Requirement: Quality result isolation

Forge SHALL scope policy execution to the selected project and redact credentials from captured evidence.

#### Scenario: Quality result isolation success

- **WHEN** two projects run quality checks
- **THEN** each registry observation references only its own execution

#### Scenario: Quality result isolation failure

- **WHEN** an external finding contains a credential-like value
- **THEN** the stored and displayed evidence redacts it

#### Scenario: Quality result isolation boundary

- **WHEN** a previous successful observation is older than the current source state
- **THEN** doctor marks it stale and does not treat it as current verification

