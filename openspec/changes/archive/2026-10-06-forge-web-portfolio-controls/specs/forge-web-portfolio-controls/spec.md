# forge-web-portfolio-controls Specification

## Purpose

Expose Forge-owned portfolio controls and truthful cross-project source evidence to the authenticated web operator.

## ADDED Requirements

### Requirement: Portfolio-owned metadata controls

Forge SHALL let the authenticated operator view and manage the Forge-owned portfolio metadata for managed projects — tags, relations, reviews and goals — through the existing Core contracts. Portfolio sharing (allowlist, preview and digest-bound approval) and its publication are owned by the separately specified delivery package and are therefore out of scope for these controls.

#### Scenario: Record portfolio review

- **WHEN** an operator saves a valid review for a managed project
- **THEN** Forge records the review using the Core operation and displays the updated actor-attributed value

#### Scenario: Attempt to edit imported evidence

- **WHEN** an operator attempts to edit source-owned evidence through the browser
- **THEN** Forge refuses the mutation and preserves the source snapshot unchanged

### Requirement: Cross-project evidence views

Forge SHALL provide web queries for project catalog, gaps, fleet, portable inventory, governance, analytics, provider evidence and readiness with source, freshness and actual status.

#### Scenario: Source is stale or unavailable

- **WHEN** an evidence source is stale or unavailable
- **THEN** the page shows that state, observation time when known, and retains other available results

#### Scenario: Provider has not run

- **WHEN** no explicit live provider probe has been requested
- **THEN** the page labels the provider `not_run` and performs no probe on page load

### Requirement: Privacy and evidence ownership

Forge SHALL preserve existing aggregate privacy thresholds and SHALL attribute imported observations to their source without converting missing or unknown values into healthy results.

#### Scenario: Aggregate below threshold

- **WHEN** interest evidence does not meet the configured minimum cohort threshold
- **THEN** Forge withholds the aggregate value and explains that the threshold was not met

#### Scenario: Optional provider disabled

- **WHEN** an optional provider is disabled, incompatible or unavailable
- **THEN** Forge shows the actual state and does not claim a passing result
