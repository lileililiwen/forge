# forge-publish-plugin-orchestration Specification

## Purpose
Forge is the only user-facing publish system. Manual project/folder CLI
input and verified GitHub push deliveries enter the same publish engine,
the provider is a switchable external JSON-line process implementing
`forge-publish-provider/0.1.0`, and OpenPanel and Jenkins remain optional
providers that can be enabled or disabled independently without rewriting
the contract.
## Requirements
### Requirement: One publish engine

Forge SHALL route manual project, manual folder, and verified GitHub push
triggers through the same `PublishRequest` and publish state machine.

#### Scenario: Manual project publish

- **WHEN** an operator runs `forge publish --project <id>`
- **THEN** Forge resolves the registered project and invokes the selected
  enabled provider through the publish contract

#### Scenario: Manual folder publish

- **WHEN** an operator runs `forge publish --folder <path>`
- **THEN** Forge captures the folder revision and uses the same publish path
  without requiring a separate deployment implementation

#### Scenario: Verified push publish

- **WHEN** Forge receives a valid GitHub push event for an enabled project
- **THEN** Forge publishes the exact full commit SHA through the same engine

### Requirement: Switchable providers

Forge SHALL allow each provider to be listed, enabled, disabled, and inspected.
Disabled providers MUST NOT be invoked for new publish runs.

#### Scenario: Disable OpenPanel

- **WHEN** an operator disables the OpenPanel provider
- **THEN** new OpenPanel publish requests are refused before provider execution

#### Scenario: Enable Jenkins compatibility

- **WHEN** an operator enables the Jenkins provider
- **THEN** Forge may select it for projects explicitly configured for Jenkins

### Requirement: Provider contract

Forge SHALL validate the `forge-publish-provider/0.1.0` request and response
contract, including provider identity, operation id, revision, terminal status,
health state, and redacted evidence.

#### Scenario: Malformed provider response

- **WHEN** a provider returns invalid JSON or a different contract version
- **THEN** Forge records a failed run and does not claim deployment health

### Requirement: Idempotent push retries

Forge SHALL derive an operation identity from project, provider, revision, and
trigger id and SHALL return the existing terminal result for duplicate events.

#### Scenario: Duplicate GitHub delivery

- **WHEN** the same signed push delivery is received twice
- **THEN** Forge invokes the provider at most once for that operation identity

### Requirement: Provider isolation

Forge SHALL preserve provider-local secrets, source handling, and transport
details. A disabled or unavailable provider MUST NOT prevent Forge startup or
manual publishing through another enabled provider.

#### Scenario: OpenPanel unavailable

- **WHEN** the OpenPanel provider executable is missing
- **THEN** Forge reports OpenPanel unavailable while another provider remains
  listable and selectable

