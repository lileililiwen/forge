## MODIFIED Requirements

### Requirement: Semantic reusable unit contracts

Forge SHALL register meaningful components with explicit inputs/outputs, tests, versions, compatibility and deterministic installation, linked to features, patterns and capabilities. A resolution SHALL report the plan it produced and SHALL NOT record a completed resolution, because a resolution installs nothing.

#### Scenario: Semantic reusable unit contracts success

- **WHEN** a PaginatedQuery or IdempotencyGuard component satisfies the contract
- **THEN** its version can be discovered and resolved for supported profiles

#### Scenario: Semantic reusable unit contracts failure

- **WHEN** an asset is merely a language primitive or lacks required contracts
- **THEN** registration rejects it with specific missing criteria

#### Scenario: Semantic reusable unit contracts boundary

- **WHEN** two stacks implement the same semantic capability
- **THEN** each preserves its own implementation while exposing the shared contract

#### Scenario: A resolved component plan is not an installation

- **WHEN** a resolution produces a plan for a profile
- **THEN** the journal records a planned resolution, the detail names
  the run, and the plan is not recorded as a completed installation
