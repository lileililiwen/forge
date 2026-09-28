# forge-publish-observability-revision-containers Specification (delta)

## ADDED Requirements

### Requirement: Phase-visible publish lifecycle

Forge SHALL persist and project every published project's `build` and
`run` phase evidence independently so `forge deploy status` answers
"did the build, the run, or both fail?" without contacting the
provider again.

#### Scenario: Build success records build=succeeded and run=succeeded

- **WHEN** a provider returns `status=done` with
  `build_status=succeeded` and `run_status=succeeded`
- **THEN** the published journal row carries both phase statuses and
  the projected state in `forge deploy status` reports the project as
  fully verified

#### Scenario: Build failure is not masked by run success

- **WHEN** a provider returns `status=done` with
  `build_status=failed` and `run_status=succeeded`
- **THEN** Forge persists `build=failed`, `run=succeeded` verbatim and
  status does NOT report the project as fully verified

#### Scenario: Run failure preserves build success

- **WHEN** a provider returns `status=failed` with
  `build_status=succeeded` and `run_status=failed`
- **THEN** Forge persists `build=succeeded`, `run=failed` and the
  aggregate state is `failed`; the build evidence remains visible

#### Scenario: Missing phase evidence is never a verified success

- **WHEN** a provider returns a successful terminal response but
  omits `build_status` and `run_status`
- **THEN** Forge persists the row with no phase evidence and the
  status projection reports the project as `unknown`, never
  `succeeded`

#### Scenario: Provider revision is a full 40-character hex SHA

- **WHEN** Forge builds the `PublishProviderRequest` for a project
- **THEN** the `revision` field is exactly 40 hexadecimal characters
  taken from the project's committed Git HEAD; an empty, short, long
  or non-hex revision is refused with `error[publish-invalid]`
  before the provider is invoked

#### Scenario: Forge accepts the additive response fields

- **WHEN** a provider terminal response carries optional `revision`,
  `build_status`, `run_status`, or `container_identity` fields
- **THEN** Forge validates the field shapes (40-char hex revision,
  phase vocabulary from
  `succeeded`/`failed`/`not_started`/`unknown`, non-empty bounded
  container identity) and persists the values on the journal row

### Requirement: Revision-bound container identity

The published revision SHALL be visible in the Docker / Compose
runtime identity so an operator inspecting `docker ps` can identify
the exact commit a running container represents.

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

#### Scenario: Status projection surfaces revision and identity

- **WHEN** `forge deploy status --queue <id>` is queried for a
  completed fleet
- **THEN** every entry exposes its `revision`,
  `build_status`, `run_status`, and `container_identity` fields
  through both the JSON document and the human renderer; missing
  values render as `unknown`, never as a verified success

### Requirement: Phase-bounded progress events

Provider progress events SHALL carry the bounded
`phase=build|run|complete` vocabulary so operators see exactly which
phase the runtime is currently executing.

#### Scenario: Build phase progress is visible

- **WHEN** the provider emits `phase=build status=started detail=...`
- **THEN** Forge forwards the bounded detail to the operator and
  updates the journal `build_status` when the corresponding terminal
  event arrives

#### Scenario: Legacy phase names are refused

- **WHEN** a provider emits `phase=build-and-run`,
  `phase=preflight`, `phase=transfer`, `phase=verify`,
  `phase=routing`, or `phase=completed`
- **THEN** Forge classifies the event as `Malformed` and surfaces the
  legacy phase name to the operator; the progress event never
  silently advances the run phase

#### Scenario: Run phase progress is visible

- **WHEN** the provider emits `phase=run status=started detail=...`
- **THEN** Forge forwards the bounded detail and updates the journal
  `run_status` when the corresponding terminal event arrives
