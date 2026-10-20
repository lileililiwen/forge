# single-delivery-lane Specification

## Purpose
Two lanes put a built artifact on a target, and they decide the opposite
things about verification. `forge delivery` refuses to ship without fresh,
passing, full-profile Forge Gate evidence bound to the revision it is about to
deliver. `forge deploy` applies the same artifact to the same kind of target
with no evidence at all, so an unverified build can reach a host through the
lane the owner uses every day. This capability gives the delivery lanes one
rule with one owner: no artifact reaches a target without fresh, passing,
full-profile Gate evidence for the revision being delivered, and both lanes
refuse for the same reason when the evidence is missing, stale, a rehearsal,
or belongs to another project.

## ADDED Requirements

### Requirement: One owner decides whether a project may be delivered

Forge SHALL decide whether an artifact may be delivered through one shared
rule. Every delivery lane SHALL use that rule and SHALL NOT keep a private
copy of it.

#### Scenario: The delivery lanes refuse for the same reason

- **WHEN** the delivery lane and the deploy lane evaluate the same project
  with the same Gate evidence
- **THEN** they reach the same verdict, and when the evidence is refused they
  name the same reason

#### Scenario: The rule has a single owner

- **WHEN** the refusal behaviour of the delivery lanes is inspected
- **THEN** the rule is implemented once and both lanes call it

### Requirement: A delivery requires fresh, passing, full-profile Gate evidence

Forge SHALL refuse a delivery unless the project has Gate evidence that uses a
supported contract, comes from the full profile, is not a rehearsal record,
belongs to that project, and is bound to the source revision being delivered.

#### Scenario: A delivery without Gate evidence is refused

- **WHEN** a delivery is requested for a project with no Gate evidence
- **THEN** the delivery is refused, the reason names the project, and the
  reason tells the owner to run the Gate from the project directory

#### Scenario: A delivery with a rehearsal record is refused

- **WHEN** the only Gate evidence for a project is a dry-run rehearsal
- **THEN** the delivery is refused, because a rehearsal is not evidence

#### Scenario: A delivery with stale evidence is refused

- **WHEN** the Gate evidence for a project is bound to a revision other than
  the one being delivered
- **THEN** the delivery is refused and the reason names the observed revision
  state

#### Scenario: A delivery with failed evidence is refused

- **WHEN** the Gate evidence for a project did not pass
- **THEN** the delivery is refused and the reason names the observed aggregate

#### Scenario: A delivery with another project's evidence is refused

- **WHEN** the Gate evidence in a project directory belongs to a different
  project
- **THEN** the delivery is refused and the reason names the observed project

#### Scenario: A delivery with a partial profile is refused

- **WHEN** the Gate evidence for a project was produced by a profile other
  than the full profile
- **THEN** the delivery is refused and the reason names the observed profile

### Requirement: The refusal precedes every mutation

Forge SHALL evaluate the delivery rule before the first side effect of a
delivery, so a refused delivery leaves the target, the recorded delivery state
and the operation journal unchanged.

#### Scenario: A refused delivery changes nothing

- **WHEN** a delivery is refused because the Gate evidence is not satisfied
- **THEN** no target is contacted, no delivery state is written and no
  operation row records a delivered artifact

#### Scenario: A rehearsal is not a delivery

- **WHEN** a delivery is run as a dry run
- **THEN** it performs no remote change and does not require Gate evidence

### Requirement: The deployment plan reports the delivery rule

Forge SHALL report in the deployment plan whether the delivery rule is
satisfied, so the owner sees the blocker before the apply step.

#### Scenario: The plan names an unsatisfied rule

- **WHEN** a deployment plan is produced for a project whose Gate evidence is
  not satisfied
- **THEN** the plan is not ready and the plan names the reason

#### Scenario: The plan reports a satisfied rule

- **WHEN** a deployment plan is produced for a project whose Gate evidence is
  satisfied
- **THEN** the plan is ready and the plan reports that the rule is satisfied
