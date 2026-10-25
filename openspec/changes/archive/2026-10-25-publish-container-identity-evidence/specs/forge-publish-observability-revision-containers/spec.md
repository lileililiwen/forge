## MODIFIED Requirements

### Requirement: Revision-bound container identity

The published revision SHALL be visible in the Docker / Compose
runtime identity so an operator inspecting `docker ps` can identify
the exact commit a running container represents. Forge SHALL compose
that identity only for a response that published: a response that
published nothing and reports no `container_identity` of its own
SHALL carry no identity at all.

#### Scenario: Canonical compose identity uses the 12-character SHA prefix

- **WHEN** Forge composes the runtime identity for a project with
  revision `0123456789abcdef0123456789abcdef01234567`
- **THEN** the Compose project / container identity is
  `forge-<project>-0123456789ab` (the first 12 hex characters); the
  full 40-character SHA stays in Forge state

#### Scenario: Provider without container_identity synthesizes it from revision

- **WHEN** a provider returns a successful response without a
  `container_identity` field
- **THEN** Forge persists the row with the canonical
  `forge-<project>-<sha12>` identity computed from the recorded
  revision so legacy providers are not silently absent from
  observability

#### Scenario: A response that published nothing carries no container

- **WHEN** a provider returns a response that did not complete the
  publish and reports no `container_identity` field
- **THEN** Forge persists the row without a container identity, the
  response carries none, and the status projection renders it as
  `unknown`

#### Scenario: Status projection surfaces revision and identity

- **WHEN** `forge deploy status --queue <id>` is queried for a
  completed fleet
- **THEN** every entry exposes its `revision`,
  `build_status`, `run_status`, and `container_identity` fields
  through both the JSON document and the human renderer; missing
  values render as `unknown`, never as a verified success
