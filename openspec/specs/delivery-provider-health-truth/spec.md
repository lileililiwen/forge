# delivery-provider-health-truth Specification

## Purpose
The delivery status projection states the provider's own answer: a promote whose provider reported a health other than healthy is degraded rather than healthy, a provider that reported healthy stays healthy, and a verb that reported no health keeps the existing verdict instead of an invented one.
## Requirements
### Requirement: The delivery phase states the provider's own health

Forge SHALL project a delivery's `phase` from the health the provider
reported for the verb, and SHALL report `degraded` when that health is
anything other than `healthy`. Forge SHALL NOT report `healthy` for a
verb whose provider reported an unhealthy state, and the projection's
evidence SHALL carry the provider's answer rather than a verdict the
provider did not give.

#### Scenario: A provider that reports unhealthy

- **WHEN** the latest promote row's provider reported a health other
  than `healthy`
- **THEN** the projection reports `phase=degraded`

#### Scenario: A provider that reports healthy

- **WHEN** the latest promote row's provider reported `healthy`
- **THEN** the projection reports `phase=healthy`

#### Scenario: A provider that reported no health

- **WHEN** the latest promote row carries no provider health
- **THEN** the projection keeps its existing verdict and does not
  manufacture one from the absence

### Requirement: The delivery projection is pinned

Forge SHALL pin the delivery projection's phase to the provider's answer
in the repository's contract check, so a provider that reports itself
unhealthy cannot be projected as healthy again.

#### Scenario: The contract check reads the projection

- **WHEN** the repository's contract check drives the delivery workflow
  against a provider that reports `healthy` and against one that reports
  `degraded`
- **THEN** it fails when the projected phase does not match the
  provider's answer
