## ADDED Requirements

### Requirement: Traceable spec generation

Forge SHALL implement spec generate with explicit project selection and generate bounded proposals from capability gaps, policy findings and semantic upgrade conflicts.

#### Scenario: Traceable spec generation success

- **WHEN** a semantic finding is selected
- **THEN** the generated spec names the project, source revision, findings, acceptance scenarios and dependencies

#### Scenario: Traceable spec generation failure

- **WHEN** findings target different projects or mutually incompatible resolutions
- **THEN** generation rejects the ambiguous request with no misleading combined spec

#### Scenario: Traceable spec generation boundary

- **WHEN** the same unchanged finding already has an active spec
- **THEN** generation reports the existing spec rather than creating a duplicate

### Requirement: Remediation routing

Forge SHALL distinguish supported deterministic fixes, semantic specs and manual intervention and require tests and quality evidence before a remediation is complete.

#### Scenario: Remediation routing success

- **WHEN** a known deterministic fix passes its validators and quality checks
- **THEN** the finding records the fix and evidence

#### Scenario: Remediation routing failure

- **WHEN** a fix fails validation
- **THEN** the finding stays unresolved with failure and recovery information

#### Scenario: Remediation routing boundary

- **WHEN** a finding requires manual judgment
- **THEN** Forge records manual status without claiming an AI fix or changing the project
