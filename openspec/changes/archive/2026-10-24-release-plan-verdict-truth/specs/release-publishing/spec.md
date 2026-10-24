## MODIFIED Requirements

### Requirement: Verified release preparation

Forge SHALL implement forge release preparation that binds semver, changelog, source revision, configured documentation, the Gate evidence for the captured revision and doctor/test/DriftWatch evidence before release side effects. The DriftWatch check SHALL judge the policy report with the same verdict the doctor reaches for that report: a report whose applicable rule failed, a report that was produced against an older source revision, and a report in which no rule applied SHALL each block the release, and only a report in which every applicable rule passed SHALL be described as healthy. A preparation releases nothing, and its registry journal row SHALL record the plan it computed rather than a completed release.

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

#### Scenario: A preparation is journalled as a plan

- **WHEN** a release preparation is recorded in the registry journal
- **THEN** the row records a planned run when the checks pass and a blocked run when a check blocks the plan, and never a completed release
