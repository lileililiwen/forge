## ADDED Requirements

### Requirement: The Gate check always applies to a release

Forge SHALL capture a Gate check for every release plan, whether or not the
project's manifest declares one. Forge SHALL treat the Gate evidence as part
of the release contract rather than as an optional check kind, and the plan
SHALL report the Gate check as applicable.

#### Scenario: A release plan always reports the Gate check

- **WHEN** an owner prepares a release for a project whose manifest declares no
  release check at all
- **THEN** the plan carries a Gate check row, that row is applicable, and the
  plan is not ready until the Gate evidence satisfies the shared delivery rule

#### Scenario: A manifest cannot turn the Gate check off

- **WHEN** a project manifest declares `kind: gate` in `release.checks`
- **THEN** the manifest is refused, and the refusal names that the Gate check
  always applies to a release

#### Scenario: The declared checks still run

- **WHEN** a project manifest declares the `doctor` and `test` checks
- **THEN** both are captured as before, and the Gate check is captured on top
  of them

### Requirement: The release lane uses the shared delivery rule

Forge SHALL decide whether the Gate evidence backs a release by calling the
shared delivery rule that the delivery and deploy lanes call, and SHALL NOT
keep a private copy of that decision. Forge SHALL report the rule's verdict
and reason unchanged.

#### Scenario: The release refuses for the same reason the delivery lane refuses

- **WHEN** the Gate evidence is absent, belongs to another revision, was
  produced by a rehearsal, was produced by a non-full profile, belongs to
  another project, or did not pass
- **THEN** the release refuses, and its reason is the shared rule's own reason

#### Scenario: The release passes when the shared rule passes

- **WHEN** the Gate evidence uses a supported contract, comes from the full
  profile, is not a rehearsal, belongs to the project, is fresh for the
  captured revision and has a passed aggregate
- **THEN** the Gate check passes

#### Scenario: The refusal names the evidence the owner must produce

- **WHEN** the shared rule refuses
- **THEN** the release's reason names the evidence that is missing or wrong,
  and how to produce it

### Requirement: The apply step re-checks the Gate evidence

Forge SHALL re-evaluate the Gate evidence in the apply step, against the
revision it is about to release, and SHALL refuse the release before any stage
runs when the shared delivery rule is not satisfied for that revision.

#### Scenario: The apply step refuses a stale release

- **WHEN** an owner applies a release whose Gate evidence is bound to another
  revision than the one being released
- **THEN** the apply step refuses before the commit stage, and no stage is
  delivered

#### Scenario: The apply step refuses an unverified release

- **WHEN** an owner applies a release for a project with no Gate evidence
- **THEN** the apply step refuses with a release-check failure, and the
  repository is left without a tag and without a publication
