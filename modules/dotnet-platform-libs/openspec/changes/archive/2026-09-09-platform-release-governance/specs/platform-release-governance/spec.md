## ADDED Requirements

### Requirement: Pull requests SHALL validate repository quality

The repository SHALL run restore, Release build, tests, `git diff --check`, and strict OpenSpec validation for changes affecting source, tests, package metadata, documentation, or OpenSpec artifacts.

#### Scenario: Source pull request passes quality gates
- **WHEN** a pull request changes a production package
- **THEN** CI restores and serially builds, tests, validates OpenSpec artifacts, and reports each gate separately

#### Scenario: Quality gate fails
- **WHEN** any required command returns a non-zero exit code
- **THEN** CI SHALL fail without presenting the change as release-ready

### Requirement: Releases SHALL validate packed artifacts

The release workflow SHALL pack selected production packages and run consumer conformance tests against those package files rather than repository project references.

#### Scenario: Packed consumer succeeds
- **WHEN** a release candidate is packed
- **THEN** the conformance fixture restores the candidate packages from a local feed and verifies registration, replacement, health, and failure behavior

### Requirement: Public API compatibility SHALL be reviewed

The repository SHALL compare public API surfaces with the previous approved baseline and SHALL require an explicit reviewed decision for breaking changes.

#### Scenario: Compatible release
- **WHEN** a public API adds a backward-compatible member
- **THEN** the compatibility check passes and the baseline can be updated in the release change

#### Scenario: Breaking release
- **WHEN** a public API removes or changes a public member
- **THEN** CI fails unless the change includes an explicit breaking-change decision and migration documentation

### Requirement: Release audits SHALL be observable

Release automation SHALL report package versions, target frameworks, dependency vulnerabilities, generated artifacts, and any skipped or unavailable checks.

#### Scenario: Audit service unavailable
- **WHEN** vulnerability or signing evidence cannot be obtained
- **THEN** the release SHALL be marked unverified rather than passing silently
