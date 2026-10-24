## MODIFIED Requirements

### Requirement: Shared project and lifecycle views

Forge SHALL provide the planned portal sections using shared API resources, including project identity, maturity, quality, agent, deployment and documentation status. A portal section SHALL state a status only from an observation it made: a section that observed no verdict SHALL report `unknown` and never `ok`, a section that could not observe its source SHALL report `unavailable`, and every entry's evidence SHALL name what the portal looked at.

#### Scenario: Shared project and lifecycle views success

- **WHEN** an authorized user opens a project
- **THEN** the portal displays API-backed current observations and navigable related resources

#### Scenario: Shared project and lifecycle views failure

- **WHEN** the API rejects access or is unavailable
- **THEN** the portal displays the actual access or availability error without cached success claims

#### Scenario: Shared project and lifecycle views boundary

- **WHEN** a status observation is stale or unknown
- **THEN** the view labels that state and retains its observation timestamp

#### Scenario: A section that observed no verdict is not a pass

- **WHEN** a portal section has no observed verdict for a project
- **THEN** the section reports `unknown` rather than `ok`, and its entries name what the portal looked at
