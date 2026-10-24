## ADDED Requirements

### Requirement: The portal states a policy verdict only when it observed one

Forge SHALL state a policy verdict for a project in the control-plane
portal only when the portal observed that verdict. A project whose
policy plane was never evaluated SHALL be reported as `unknown`, never
as `ok`, and a project whose directory could not be read SHALL be
reported as `unavailable`. The Policies entry SHALL name what the
portal observed — the DriftWatch policy artifacts in the project
directory — and, when no verdict was observed, SHALL name the command
that produces one. The portal and the doctor SHALL agree on what a
DriftWatch policy artifact is.

#### Scenario: A policy that was never evaluated is unknown

- **WHEN** the portal renders the Policies section for a project with no
  recorded policy verdict
- **THEN** the entry is reported as `unknown`, its evidence names the
  policy artifacts the portal found, and it says that an unevaluated
  policy is not a pass

#### Scenario: A recorded policy verdict is reported

- **WHEN** the portal renders the Policies section for a project with a
  recorded policy verdict
- **THEN** the entry reports that verdict and names it in its evidence

#### Scenario: An unreadable project is unavailable

- **WHEN** the portal renders the Policies section for a project whose
  directory cannot be read
- **THEN** the entry is reported as `unavailable` and says that the
  policy plane was not observed

#### Scenario: One list of policy artifacts

- **WHEN** the doctor and the portal both look for a project's DriftWatch
  policy artifacts
- **THEN** both read the same detector, so the two agree on what a
  policy artifact is
