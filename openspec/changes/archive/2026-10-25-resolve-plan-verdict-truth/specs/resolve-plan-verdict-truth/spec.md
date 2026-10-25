## ADDED Requirements

### Requirement: A resolution names the plan it produced

Forge SHALL judge a resolution with one rule: a resolution that produced
a plan is a planned run, and a request the resolver could not satisfy is
a rejected run. A resolution SHALL NOT be recorded as a completed
resolution, because it installs nothing. Every journal row for a
resolution SHALL follow that one rule rather than a private copy of it,
and the row's detail SHALL name the run. The component resolver and the
UI pattern resolver SHALL read the same rule.

#### Scenario: A resolution that produced a plan is planned

- **WHEN** a resolution produces at least one step
- **THEN** the journal row records a planned resolution, the detail
  names the run as `planned`, and the command's output names it too

#### Scenario: A resolution that satisfied nothing is rejected

- **WHEN** a resolution produces no step and names what it could not
  satisfy
- **THEN** the journal row records a rejected resolution and the detail
  names the run as `rejected`

#### Scenario: One rule, every resolution writer

- **WHEN** the component resolver and the UI pattern resolver record a
  resolution
- **THEN** both read the one rule for the run they describe
