## ADDED Requirements

### Requirement: The fleet provider lane names what its run did

Forge SHALL judge a fleet provider publish with the one publish rule:
a run that attempted no write is a plan, a run the provider reported
healthy is a delivery, and a run the provider did not report healthy is
a failure. The fleet provider lane SHALL read its health from the
provider's answer alone, SHALL record the run's verdict in the registry
journal row instead of a private copy of the rule, and SHALL NOT record
a container identity for a run that published nothing. A plan SHALL NOT
be counted as a failure.

#### Scenario: A planned fleet provider publish is not a completed publish

- **WHEN** a fleet publish runs the provider lane as a dry run
- **THEN** the fleet summary reports the project as not healthy with the
  verdict `planned`, the journal row records a planned publish, and the
  row names no container identity

#### Scenario: A delivered fleet provider publish is unchanged

- **WHEN** a fleet publish runs the provider lane and the provider
  reports the target healthy
- **THEN** the fleet summary reports the project as healthy, the journal
  row records a completed publish, and the row names the provider's
  container identity

#### Scenario: A fleet provider publish that is not healthy fails

- **WHEN** a fleet publish runs the provider lane and the provider
  reports no healthy target
- **THEN** the journal row records a failed publish and the fleet counts
  the project as a failure

#### Scenario: One rule, every publish writer

- **WHEN** the publish pipeline, the fleet lane, the fleet provider lane
  and the fleet exec lane record a publish
- **THEN** all four read the one publish rule for the run they describe
