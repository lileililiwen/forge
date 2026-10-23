## MODIFIED Requirements

### Requirement: Canonical and derivative documentation

Forge SHALL implement docs translate with explicit locale enablement, derivative paths, source hashes and stale status on source changes. The report SHALL carry one aggregate verdict over its outcomes, and that verdict SHALL name the outcomes' review status.

#### Scenario: Canonical and derivative documentation success

- **WHEN** translation of an enabled locale completes against an unchanged source
- **THEN** the derivative records its source hash and review status

#### Scenario: Canonical and derivative documentation failure

- **WHEN** a derivative path resolves to the source file or outside the permitted project
- **THEN** translation fails before writing

#### Scenario: Canonical and derivative documentation boundary

- **WHEN** a locale is disabled
- **THEN** automatic workflows do not invoke its provider or create output

#### Scenario: The report's verdict names the review status

- **WHEN** a locale's translation completes but its derivative needs review
- **THEN** the report's verdict names that review status, the report is not healthy, and no surface calls the run complete
