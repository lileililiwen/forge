# release-publishing Specification

## Purpose
Verified release preparation and staged publishing: semver/changelog/revision-bound plans with doctor/test/DriftWatch gates, confirm-gated per-stage apply (commit, tag, push, mirror, package, container, docs, notes) with tag-conflict refusal and retry-safe skipped stages.
## Requirements
### Requirement: Verified release preparation

Forge SHALL implement forge release preparation that binds semver, changelog, source revision, configured documentation and doctor/test/DriftWatch evidence before release side effects.

#### Scenario: Verified release preparation success

- **WHEN** all required checks pass for the captured revision
- **THEN** the reviewable plan becomes eligible for commit and tagging

#### Scenario: Verified release preparation failure

- **WHEN** a required check fails, is unavailable or belongs to an older revision
- **THEN** release execution blocks before tag or publication

#### Scenario: Verified release preparation boundary

- **WHEN** the project has no translation locale configured
- **THEN** release omits translation while retaining the other configured checks

### Requirement: Resumable multi-destination publication

Forge SHALL execute authorized commit, tag, primary push, mirror, package, container and release-note stages with per-stage records and safe retry; forge publish SHALL use the same verified release identity and publication stages without bypassing release checks.

#### Scenario: Resumable multi-destination publication success

- **WHEN** every selected release stage completes
- **THEN** the release record links immutable versions and provider receipts

#### Scenario: Resumable multi-destination publication failure

- **WHEN** a package publishes but the container push fails
- **THEN** the release is partial and retry does not republish or overwrite the package

#### Scenario: Resumable multi-destination publication boundary

- **WHEN** a tag or immutable version already exists with different content
- **THEN** the operation refuses replacement and reports the identity conflict

