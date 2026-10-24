# release-publishing Specification

## Purpose
Verified release preparation and staged publishing: semver/changelog/revision-bound plans with the Gate evidence for the captured revision and doctor/test/DriftWatch gates, the DriftWatch gate judged with the doctor's own verdict on the policy report, confirm-gated per-stage apply (commit, tag, push, mirror, package, container, docs, notes) with tag-conflict refusal and retry-safe skipped stages, and one gate that keeps a dry run free of side effects.
## Requirements
### Requirement: Verified release preparation

Forge SHALL implement forge release preparation that binds semver, changelog, source revision, configured documentation, the Gate evidence for the captured revision and doctor/test/DriftWatch evidence before release side effects. The DriftWatch check SHALL judge the policy report with the same verdict the doctor reaches for that report: a report whose applicable rule failed, a report that was produced against an older source revision, and a report in which no rule applied SHALL each block the release, and only a report in which every applicable rule passed SHALL be described as healthy.

#### Scenario: Verified release preparation success

- **WHEN** all required checks pass for the captured revision
- **THEN** the reviewable plan becomes eligible for commit and tagging

#### Scenario: Verified release preparation failure

- **WHEN** a required check fails, is unavailable or belongs to an older revision
- **THEN** release execution blocks before tag or publication

#### Scenario: Verified release preparation boundary

- **WHEN** the project has no translation locale configured
- **THEN** release omits translation while retaining the other configured checks

#### Scenario: A policy report the doctor does not call healthy

- **WHEN** the policy report warns, applies to no rule, or belongs to an
  older source revision
- **THEN** the DriftWatch check carries the doctor's verdict for that
  report, and the plan names the rules behind it instead of calling the
  report healthy

### Requirement: Resumable multi-destination publication

Forge SHALL execute authorized commit, tag, primary push, mirror, package, container and release-note stages with per-stage records and safe retry; forge publish SHALL use the same verified release identity and publication stages without bypassing release checks or the Gate evidence. A stage SHALL write only when the release is confirmed and the run is not a dry run, and a stage that may not write SHALL record a status that is not a delivery together with a note that names the reason it did not write.

#### Scenario: Resumable multi-destination publication success

- **WHEN** every selected release stage completes
- **THEN** the release record links immutable versions and provider receipts

#### Scenario: Resumable multi-destination publication failure

- **WHEN** a package publishes but the container push fails
- **THEN** the release is partial and retry does not republish or overwrite the package

#### Scenario: Resumable multi-destination publication boundary

- **WHEN** a tag or immutable version already exists with different content
- **THEN** the operation refuses replacement and reports the identity conflict

#### Scenario: A dry run applies no stage

- **WHEN** a release is applied as a dry run
- **THEN** no stage writes to the repository, the host or a provider, and every stage record names why it did not write

