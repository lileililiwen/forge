# publish-fleet-dry-run-verdict-truth Specification

## Purpose
The fleet provider lane names what its run did: one publish rule judges every provider publish, a planned publish is never recorded as a completed publish, a run that published nothing records no container identity, and the fleet summary counts every run from its verdict.
## Requirements
### Requirement: The fleet provider lane names what its run did

Forge SHALL judge a fleet provider publish with the one publish rule:
a run that attempted no write is a plan, a run the provider reported
healthy is a delivery, and a run the provider did not report healthy is
a failure. The fleet provider lane SHALL read its health from the
provider's answer alone, SHALL record the run's verdict in the registry
journal row instead of a private copy of the rule, and SHALL NOT record
a container identity for a run that published nothing. A plan SHALL NOT
be counted as a failure. The fleet summary SHALL count every run from
its verdict: a delivery is a success, a plan is a plan, and a failure
is a failure; a plan SHALL NOT be counted as a success and a delivery
SHALL NOT be counted as nothing.

#### Scenario: A planned fleet provider publish is not a completed publish

- **WHEN** a fleet publish runs the provider lane as a dry run
- **THEN** the fleet summary reports the project as not healthy with the
  verdict `planned`, the journal row records a planned publish, and the
  row names no container identity

#### Scenario: A planned fleet publish is not counted as a success

- **WHEN** a fleet publish counts a run whose verdict is `planned`
- **THEN** the summary counts it as a plan and not as a success, and
  the summary names the planned runs

#### Scenario: A delivered fleet provider publish is unchanged

- **WHEN** a fleet publish runs the provider lane and the provider
  reports the target healthy
- **THEN** the fleet summary reports the project as healthy, the journal
  row records a completed publish, and the row names the provider's
  container identity

#### Scenario: A delivered fleet publish is counted as a success

- **WHEN** a fleet publish counts a run whose verdict is `delivered`
- **THEN** the summary counts it as a success, in every fleet lane

#### Scenario: A fleet provider publish that is not healthy fails

- **WHEN** a fleet publish runs the provider lane and the provider
  reports no healthy target
- **THEN** the journal row records a failed publish and the fleet counts
  the project as a failure

#### Scenario: One rule, every publish writer

- **WHEN** the publish pipeline, the fleet lane, the fleet provider lane
  and the fleet exec lane record a publish
- **THEN** all four read the one publish rule for the run they describe
