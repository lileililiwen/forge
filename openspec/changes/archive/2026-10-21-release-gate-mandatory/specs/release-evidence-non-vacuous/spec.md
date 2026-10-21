## MODIFIED Requirements

### Requirement: A release plan is ready only with applicable evidence

Forge SHALL treat a release plan as ready only when the Gate evidence
satisfies the shared delivery rule for the captured revision and every
applicable release check passes. Forge SHALL NOT treat the absence of an
applicable check as a pass, and the plan SHALL name the missing evidence when
it is not ready.

#### Scenario: A release with no declared check is refused

- **WHEN** an owner prepares a release for a project whose manifest declares no
  release check
- **THEN** the plan is not ready, the plan names the missing Gate evidence,
  and the plan reports no passing check count

#### Scenario: A release with a passing check is ready

- **WHEN** the Gate evidence satisfies the shared delivery rule for the
  captured revision and every applicable release check passes
- **THEN** the plan is ready and reports the count of passing checks

#### Scenario: A release with a failing check is refused

- **WHEN** an applicable check fails, is unavailable or is stale
- **THEN** the plan is not ready and the reason names the failing check and its
  status

### Requirement: The apply step cannot apply a release without evidence

Forge SHALL re-evaluate the captured checks in the apply step and SHALL refuse
the release before any stage runs when the Gate evidence does not satisfy the
shared delivery rule for the applied revision.

#### Scenario: The apply step refuses a release with no applicable check

- **WHEN** an owner applies a release for a project whose Gate evidence does
  not satisfy the shared delivery rule for the applied revision
- **THEN** the apply step refuses with a release-check failure, and no stage is
  delivered

#### Scenario: The apply step still refuses a failing check

- **WHEN** an applicable check is not passing at the applied revision
- **THEN** the apply step refuses before the commit stage

### Requirement: The refusal tells the owner what to declare

Forge SHALL name the release check kinds a project can declare and the Gate
evidence a release requires in the refusal, so the owner can produce the
evidence without reading the source.

#### Scenario: The reason names the available check kinds

- **WHEN** a release is refused for having no applicable check
- **THEN** the reason names `doctor`, `test` and `driftwatch` as the kinds
  `release.checks` accepts

#### Scenario: The reason names the Gate evidence the release requires

- **WHEN** a release is refused because the Gate evidence does not satisfy the
  shared delivery rule
- **THEN** the reason names the missing or wrong evidence and how to produce
  it
