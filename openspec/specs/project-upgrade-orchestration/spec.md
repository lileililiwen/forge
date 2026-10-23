# project-upgrade-orchestration Specification

## Purpose
Deterministic upgrade planning and fleet orchestration: pinned plans with compatibility checks and recovery implications, semantic-conflict handoff to spec generation, per-project failure isolation, no-op on already-satisfied upgrades, and a fleet verdict that distinguishes a judged fleet from one whose projects could not be judged.
## Requirements
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

Forge SHALL implement upgrade --all with explicit project selection, per-project journals and distinct success, failure, blocked, skipped and unavailable results. A project that could not be judged — its registry record was unreadable, or its path was unavailable — SHALL be reported as `unavailable` rather than `skipped`, and the fleet's verdict SHALL be `unavailable` rather than `healthy`.

#### Scenario: Isolated fleet outcomes success

- **WHEN** two eligible projects complete their migrations
- **THEN** each records its new versions and validation evidence

#### Scenario: Isolated fleet outcomes failure

- **WHEN** one project fails midway through a fleet run
- **THEN** its partial state and recovery steps are reported without marking the fleet wholly successful

#### Scenario: Isolated fleet outcomes boundary

- **WHEN** a previous run is retried
- **THEN** completed steps are not blindly repeated and changed preconditions cause re-planning

#### Scenario: A project that could not be judged is unavailable

- **WHEN** a selected project's registry record was unreadable, or its path was unavailable
- **THEN** that project's result is `unavailable`, the fleet's verdict is `unavailable`, and the fleet is not reported as healthy

#### Scenario: One verdict, every surface

- **WHEN** the fleet report is rendered for a human, rendered as JSON, or used to decide the command's exit code
- **THEN** all three read the same verdict

