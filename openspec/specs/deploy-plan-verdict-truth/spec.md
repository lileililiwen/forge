# deploy-plan-verdict-truth Specification

## Purpose
The deploy journal names what the run did: one rule in the deploy model turns a plan, a blocked plan, a delivery, a rehearsal or a failure into the row's state, and a plan is never recorded as a completed deployment.
## Requirements
### Requirement: The deploy journal names what a plan did

Forge SHALL name what a deploy run did with one rule, in the deploy
model: a plan the shared delivery rule allows, a plan it blocks, a
deploy that applied the artifact, a deploy that applied nothing and a
deploy whose stage did not complete. The registry journal row of a
deploy plan SHALL record that run, and a plan SHALL NOT record a
completed deployment, because a plan deploys nothing. Every journal row
of the deploy lane SHALL name the run in its detail, so a reader never
has to guess what the state means.

#### Scenario: A plan is not a completed deployment

- **WHEN** a deploy plan is computed for a revision the shared delivery
  rule allows
- **THEN** the journal row records a planned run, never a completed one,
  and the row's detail names the run

#### Scenario: A blocked plan stays blocked

- **WHEN** a deploy plan is computed for a revision the shared delivery
  rule blocks
- **THEN** the journal row records a blocked run, and the plan's own
  note names the rule's reason

#### Scenario: One rule, every writer

- **WHEN** the deploy plan and the deploy apply write their journal rows
- **THEN** both read the same rule for the run they describe and both
  name the run in the row's detail

#### Scenario: The applied deploy is unchanged

- **WHEN** a confirmed deploy applies the artifact and every stage
  completes
- **THEN** the journal row records a completed deploy, exactly as before
