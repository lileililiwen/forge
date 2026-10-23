## MODIFIED Requirements

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
