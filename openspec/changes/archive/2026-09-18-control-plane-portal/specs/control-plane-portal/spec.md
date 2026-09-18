## ADDED Requirements

### Requirement: Shared project and lifecycle views

Forge SHALL provide the planned portal sections using shared API resources, including project identity, maturity, quality, agent, deployment and documentation status.

#### Scenario: Shared project and lifecycle views success

- **WHEN** an authorized user opens a project
- **THEN** the portal displays API-backed current observations and navigable related resources

#### Scenario: Shared project and lifecycle views failure

- **WHEN** the API rejects access or is unavailable
- **THEN** the portal displays the actual access or availability error without cached success claims

#### Scenario: Shared project and lifecycle views boundary

- **WHEN** a status observation is stale or unknown
- **THEN** the view labels that state and retains its observation timestamp

### Requirement: Accessible controlled operations

Forge SHALL provide keyboard-accessible responsive operation flows with loading, empty, error, success and partial-result states, preserving Core review and authorization requirements.

#### Scenario: Accessible controlled operations success

- **WHEN** a user starts an authorized lifecycle operation
- **THEN** the UI displays its plan where required and tracks its operation ID to completion

#### Scenario: Accessible controlled operations failure

- **WHEN** an operation fails after a partial external write
- **THEN** the UI shows completed and failed stages plus recovery guidance

#### Scenario: Accessible controlled operations boundary

- **WHEN** the portal is absent or closed
- **THEN** the same supported operations remain available through CLI and mature MCP tools
