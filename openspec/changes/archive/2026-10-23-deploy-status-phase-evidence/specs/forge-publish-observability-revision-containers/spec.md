## MODIFIED Requirements

### Requirement: Phase-visible publish lifecycle

Forge SHALL persist and project every published project's `build` and
`run` phase evidence independently so `forge deploy status` answers
"did the build, the run, or both fail?" without contacting the
provider again. Every projected entry SHALL expose its `revision`,
`build_status`, `run_status` and `container_identity` fields, and a
value the journal row does not carry SHALL be projected as `unknown`
rather than omitted, so no entry reads as a verified success by
silence.

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

#### Scenario: Status projection surfaces revision and identity

- **WHEN** `forge deploy status --queue <id>` is queried for a
  completed fleet
- **THEN** every entry exposes its `revision`,
  `build_status`, `run_status`, and `container_identity` fields
  through both the JSON document and the human renderer; missing
  values render as `unknown`, never as a verified success
