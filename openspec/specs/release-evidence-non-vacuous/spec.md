# release-evidence-non-vacuous Specification

## Purpose
A release with no applicable check used to pass. The release engine judged readiness with "every applicable check passes", and a project that declared no `release.checks` had no applicable check at all, so the judgement was true on an empty set: the plan reported `ready: yes` with zero passing checks and the apply step walked the commit, tag, push and publication stages on a release nothing had verified. A release now needs at least one applicable check, and a release without one is refused before any stage runs, with a reason that names the evidence the owner still has to produce.
## Requirements
### Requirement: A release plan is ready only with applicable evidence

Forge SHALL treat a release plan as ready only when at least one release check
is applicable and every applicable check passes. Forge SHALL NOT treat the
absence of an applicable check as a pass, and the plan SHALL name the missing
evidence when it is not ready.

#### Scenario: A release with no declared check is refused

- **WHEN** an owner prepares a release for a project whose manifest declares no
  release check
- **THEN** the plan is not ready, the plan names the missing check, and the plan
  reports no passing check count

#### Scenario: A release with a passing check is ready

- **WHEN** at least one release check is applicable and every applicable check
  passes
- **THEN** the plan is ready and reports the count of passing checks

#### Scenario: A release with a failing check is refused

- **WHEN** an applicable check fails, is unavailable or is stale
- **THEN** the plan is not ready and the reason names the failing check and its
  status

### Requirement: The apply step cannot apply a release without evidence

Forge SHALL re-evaluate the captured checks in the apply step and SHALL refuse
the release before any stage runs when no check is applicable.

#### Scenario: The apply step refuses a release with no applicable check

- **WHEN** an owner applies a release for a project whose manifest declares no
  release check
- **THEN** the apply step refuses with a release-check failure, and no stage is
  delivered

#### Scenario: The apply step still refuses a failing check

- **WHEN** an applicable check is not passing at the applied revision
- **THEN** the apply step refuses before the commit stage

### Requirement: The refusal tells the owner what to declare

Forge SHALL name the release check kinds a project can declare in the refusal,
so the owner can produce the evidence without reading the source.

#### Scenario: The reason names the available check kinds

- **WHEN** a release is refused for having no applicable check
- **THEN** the reason names `doctor`, `test`, `driftwatch` and `gate` as the
  kinds `release.checks` accepts
