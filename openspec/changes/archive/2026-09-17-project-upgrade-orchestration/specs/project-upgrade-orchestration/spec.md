## ADDED Requirements

### Requirement: Deterministic upgrade planning

Forge SHALL produce pinned upgrade plans with compatibility checks, package/configuration/codemod/schema/replacement steps and recovery implications before applying a project or feature upgrade.

#### Scenario: Deterministic upgrade planning success

- **WHEN** a known migration path exists
- **THEN** the plan describes old and new versions, affected assets, validation and recovery

#### Scenario: Deterministic upgrade planning failure

- **WHEN** a custom edit invalidates migration preconditions
- **THEN** the project remains unchanged and a semantic-conflict handoff is emitted

#### Scenario: Deterministic upgrade planning boundary

- **WHEN** a requested upgrade is already satisfied
- **THEN** the plan reports no changes without rewriting files

### Requirement: Isolated fleet outcomes

Forge SHALL implement upgrade --all with explicit project selection, per-project journals and distinct success, failure, blocked and skipped results.

#### Scenario: Isolated fleet outcomes success

- **WHEN** two eligible projects complete their migrations
- **THEN** each records its new versions and validation evidence

#### Scenario: Isolated fleet outcomes failure

- **WHEN** one project fails midway through a fleet run
- **THEN** its partial state and recovery steps are reported without marking the fleet wholly successful

#### Scenario: Isolated fleet outcomes boundary

- **WHEN** a previous run is retried
- **THEN** completed steps are not blindly repeated and changed preconditions cause re-planning
