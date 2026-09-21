# core-http-api Specification

## Purpose
Stable HTTP/1.1 project and lifecycle operations (projects, doctor, features, upgrade, specs, agents, deployments, operations) reusing the CLI/MCP Core validation and results, with per-project OIDC sessions, confirm-gated mutations, idempotent operations and bounded request handling.
## Requirements
### Requirement: Equivalent HTTP domain behavior

Forge SHALL expose stable HTTP project and lifecycle operations using the same validation and results as CLI and MCP.

#### Scenario: Equivalent HTTP domain behavior success

- **WHEN** a permitted project doctor operation is invoked through HTTP
- **THEN** its domain findings match the same Core invocation through CLI

#### Scenario: Equivalent HTTP domain behavior failure

- **WHEN** a request contains an invalid manifest or incompatible feature
- **THEN** HTTP returns a structured validation error without bypassing Core checks

#### Scenario: Equivalent HTTP domain behavior boundary

- **WHEN** the API server is stopped
- **THEN** local CLI operations remain usable

### Requirement: Authorized asynchronous operations

Forge SHALL enforce project/action authorization and expose operation identity, progress and final outcomes for long-running requests.

#### Scenario: Authorized asynchronous operations success

- **WHEN** an authorized deployment request is accepted
- **THEN** the caller receives an operation ID and can retrieve the recorded outcome

#### Scenario: Authorized asynchronous operations failure

- **WHEN** a caller requests another unauthorized project or reuses an idempotency key for different inputs
- **THEN** the API rejects the request

#### Scenario: Authorized asynchronous operations boundary

- **WHEN** an identical accepted request is retried
- **THEN** it returns the same operation identity without repeating external side effects

