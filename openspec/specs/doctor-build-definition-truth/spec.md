# doctor-build-definition-truth Specification

## Purpose
Read-only build-definition judging in forge doctor: the expected build definition of every supported profile, a project judged against that definition with evidence, and a profile the doctor cannot judge reported as unavailable rather than as a failure of the inspected project.
## Requirements
### Requirement: The doctor's build definition covers every supported profile

Forge SHALL know the build definition of every profile the catalogue lists
as supported, and SHALL judge a project against that definition. The
expectation SHALL be the named file, the file extension or the set of
files the profile's project contains, and SHALL cover every supported
profile without exception.

#### Scenario: A supported profile's build definition is known

- **WHEN** the catalogue lists a profile as supported
- **THEN** the doctor expects that profile's build definition, and the
  repository's contract check fails naming the profile when it does not

#### Scenario: A present build definition passes

- **WHEN** doctor runs on a project whose profile's build definition is
  present
- **THEN** the `build-config` finding is `PASS` and its evidence names the
  file that was found

#### Scenario: A missing build definition is still a failure

- **WHEN** doctor runs on a project whose profile's build definition is
  missing
- **THEN** the `build-config` finding is `FAIL` and its evidence names the
  expected file

### Requirement: A profile the doctor cannot judge is unavailable

Forge SHALL report a check it cannot evaluate as unavailable rather than as
a failure of the inspected project, and SHALL name the profile in the
evidence. Forge SHALL NOT report a missing build definition for a profile
it holds no expectation for.

#### Scenario: A profile without a doctor expectation is unavailable

- **WHEN** doctor runs on a project whose profile the doctor holds no
  build definition for
- **THEN** the `build-config` finding is `unavailable` and its evidence
  names the profile

#### Scenario: The doctor does not blame the project for its own gap

- **WHEN** the doctor holds no build definition for the project's profile
- **THEN** the finding does not report a missing build definition and the
  project is not failed for it

### Requirement: The doctor's build definition table is pinned

Forge SHALL pin the expected build definition of every supported profile,
so that a profile cannot be added to the catalogue without the doctor
learning it, and so that an expectation cannot change without the
repository's contract check failing and naming the profile.

#### Scenario: The catalogue and the doctor's table agree

- **WHEN** the repository's contract check reads the catalogue and the
  doctor's table
- **THEN** every supported profile has the expected build definition the
  table pins, every expectation belongs to a supported profile, and the
  check fails naming the profile when one of these does not hold

