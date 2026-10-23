## ADDED Requirements

### Requirement: The release lane judges a policy report with the doctor's own verdict

Forge SHALL judge a policy report in the release lane with the same
verdict the doctor reaches for that report, and SHALL reach it through one
shared rule rather than a filter of its own. A report whose applicable
rule failed SHALL be a failing check, a report that was produced against
an older source revision SHALL be a stale check, and a report in which no
rule applied SHALL be an unavailable check — never a pass. A report whose
applicable rules warned and none failed SHALL NOT block the release, and
a report in which every applicable rule passed SHALL be the only report
the release lane calls healthy.

#### Scenario: A rule failed

- **WHEN** the policy report carries an applicable rule with a failing
  severity
- **THEN** the release check is a failure and the release plan is not
  ready

#### Scenario: No rule applied

- **WHEN** the policy report carries no applicable rule
- **THEN** the release check is unavailable, the release plan is not
  ready, and the evidence names that there was no policy evidence to
  release on

#### Scenario: The report belongs to an older revision

- **WHEN** the policy report was produced against an older source state
  than the one the release captures
- **THEN** the release check is stale and the release plan is not ready

#### Scenario: A rule warned

- **WHEN** the policy report carries an applicable rule with a warning
  severity and none with a failing severity
- **THEN** the release check passes and its evidence names the warning

#### Scenario: Every rule passed

- **WHEN** the policy report carries at least one applicable rule and
  every applicable rule passed
- **THEN** the release check passes and its evidence names the report
  healthy

### Requirement: A release check's evidence names what the report is

Forge SHALL state in a captured check's evidence and detail what the
evidence behind it actually is. A captured check SHALL NOT call a report
healthy unless every applicable rule in it passed, and a check that
carries warnings SHALL name them. A check's evidence SHALL name the rules
behind its verdict and SHALL NOT carry the report's own free text.

#### Scenario: The evidence of a passing report

- **WHEN** the policy report passed every applicable rule
- **THEN** the release check's evidence names the report healthy

#### Scenario: The evidence of a warning report

- **WHEN** the policy report warned on an applicable rule
- **THEN** the release check's evidence names the warning and does not
  name the report healthy

#### Scenario: The evidence of an unjudged report

- **WHEN** the policy report carried no applicable rule
- **THEN** the release check's evidence names that no rule applied, and
  does not name the report healthy

### Requirement: The release check's verdicts are pinned

Forge SHALL pin the release lane's verdicts for a policy report in the
repository's contract check, so that a report the doctor does not call
healthy cannot be captured as a healthy release check again.

#### Scenario: The contract check reads the release plan

- **WHEN** the repository's contract check prepares a release plan against
  a policy report with a failing rule, with no applicable rule, from an
  older revision, with a warning, and with every rule passed
- **THEN** it fails when the captured check's status or evidence does not
  match the shared verdict
