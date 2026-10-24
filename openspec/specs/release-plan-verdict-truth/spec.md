# release-plan-verdict-truth Specification

## Purpose
The release journal names what the run did: one rule in the release model turns a plan, a blocked plan, an applied release or an incomplete applied release into the row's state, and a preparation is never recorded as a completed release.
## Requirements
### Requirement: The release journal names what the run did

Forge SHALL name what a release run did with one rule, in the release
model: a plan the checks allow, a plan a check blocks, an applied
release and an applied release that did not complete every stage. The
registry journal row of a release SHALL record that run, and a
preparation SHALL NOT record a completed release, because a
preparation releases nothing. Every journal row of the release lane
SHALL name the run in its detail, so a reader never has to guess what
the state means.

#### Scenario: A preparation is not an applied release

- **WHEN** a release preparation computes a plan whose checks pass
- **THEN** the journal row records a planned run, never a completed one, and the row's detail names the run

#### Scenario: A blocked preparation stays blocked

- **WHEN** a release preparation computes a plan that a check blocks
- **THEN** the journal row records a blocked run, and the plan's own reason names the failing check

#### Scenario: One rule, every writer

- **WHEN** the release prepare and the release apply write their journal rows
- **THEN** both read the same rule for the run they describe and both name the run in the row's detail

#### Scenario: The applied release is unchanged

- **WHEN** a confirmed release is applied and every stage completes
- **THEN** the journal row records a completed release, exactly as before
