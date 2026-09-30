# project-to-production-workflow Specification

## Purpose
Evidence-gated staged delivery with explicit production promotion, optional post-publish Hermora onboarding, and read-only delivery status.
## Requirements
### Requirement: Evidence-gated staged delivery

Forge SHALL coordinate project checks, provider preflight, stage publication, and health verification through the existing publish provider contract (`forge-publish-provider/0.1.0`).

#### Scenario: Preflight succeeds

- **WHEN** the project is registered, the provider's `preflight` operation returns `status=succeeded` for the current source revision
- **THEN** Forge records the terminal preflight evidence under `kind=delivery.preflight` and advances `phase` to `preflighted`

#### Scenario: Preflight fails

- **WHEN** the provider returns `status != succeeded` or the binary is missing
- **THEN** Forge records the failure under `delivery.preflight` with `state=failed`, surfaces `delivery-unavailable`, and refuses the next stage

#### Scenario: Stage is healthy

- **WHEN** a `delivery.preflight` row is terminal for the same `(project, revision)`, the user confirms the stage with `--confirm-operation-id <op_id>` (API: `confirm_operation_id`), and the provider's `publish` operation returns terminal `build_status=succeeded` and `run_status=succeeded`
- **THEN** Forge records the immutable revision, the provider's `container_identity`, and the terminal stage health evidence; `phase` advances to `stage-healthy`

#### Scenario: Stage health is unhealthy

- **WHEN** the provider's `publish` operation reports `build_status=failed` or `run_status=failed`, or the stage verify returns unhealthy
- **THEN** Forge records `state=failed` under `delivery.stage`, surfaces `delivery-unavailable`, transitions `phase` to `stage-failed`, and refuses promotion

#### Scenario: Required preflight missing

- **WHEN** the user invokes `stage` without a prior terminal `delivery.preflight` row for the same revision, or with a stale `confirm_operation_id`
- **THEN** Forge refuses with `delivery-invalid` and writes 0 bytes to stdout

### Requirement: Explicit production promotion

Forge SHALL require a revision-bound user confirmation before production publication.

#### Scenario: Confirm current revision

- **WHEN** the user confirms with `--confirm-revision <revision>` (API: `confirm_revision` body field) matching the registered source revision, and a terminal `delivery.stage` row exists for that revision in `stage-healthy`
- **THEN** Forge invokes the provider with `publish` (queue id `delivery-production-<project>-<revision-12>`), records the provider's terminal evidence under `delivery.promote`, and advances `phase` to `production → healthy`

#### Scenario: Stale or missing confirmation

- **WHEN** the user submits `--confirm-revision` with a revision that does not match the registered source revision, or omits the flag entirely
- **THEN** Forge refuses with `delivery-conflict` (mismatch) or `delivery-invalid` (omitted) before invoking the production provider; writes 0 bytes to stdout

#### Scenario: Promote after unhealthy stage

- **WHEN** the user invokes promote and the most recent terminal `delivery.stage` row for the same revision is `stage-failed`
- **THEN** Forge refuses with `delivery-conflict`, identifying the failing evidence by `operation_id`

#### Scenario: Idempotent repeat

- **WHEN** the user re-invokes `stage` or `promote` with the exact same `(project, revision, confirm)` tuple (request hash unchanged)
- **THEN** Forge re-uses the existing `op_id` and does not invoke the provider a second time

#### Scenario: Idempotency-key reuse with different request

- **WHEN** the user re-invokes a verb with the same deterministic idempotency key but a different request hash (revised confirmation)
- **THEN** Forge refuses with `idempotency-key-conflict` and surfaces both the old and the new request identifiers

### Requirement: Optional post-publish Hermora onboarding

Forge SHALL treat Hermora site enrollment as a separate, optional child operation after successful deployment, reusing the `delivery.promote` revision.

#### Scenario: Enrollment succeeds

- **WHEN** the operator invokes `hermora-retry` with `--deployment-url <url>` and `--secret-ref <ref>` (API: `deployment_url` + `secret_ref` body fields) and the `forge-delivery-hermora/0.1.0` adapter returns `status=connected`
- **THEN** Forge records the returned `site_id` and `environment_url` under `delivery.hermora` with `state=connected` and advances `phase` to `hermora-connected`; the deployment is not re-invoked

#### Scenario: Hermora is unavailable

- **WHEN** deployment is healthy but the Hermora adapter is missing, exits non-zero, returns a non-`connected` status, or exceeds its bounded wall-clock budget
- **THEN** Forge preserves the `healthy` deployment result, leaves `phase` at `hermora-pending` or `hermora-failed`, and exposes the idempotent `hermora-retry` command that does not republish

#### Scenario: Hermora retry after failure

- **WHEN** the operator re-invokes `hermora-retry` after a `hermora-failed` row for the same `(project, revision)`
- **THEN** Forge invokes the Hermora adapter once, re-uses the `op_id` only if the request hash matches, and never re-invokes the publish provider

#### Scenario: Hermora envelope is rejected

- **WHEN** the adapter returns an envelope that is not the `forge-delivery-hermora/0.1.0` contract, is not JSON, exceeds the size bound, or carries a credential-shaped value
- **THEN** Forge refuses with `delivery-invalid`, redacts the offending value, and writes 0 bytes to stdout

### Requirement: Read-only delivery status

Forge SHALL expose the project's current delivery `phase`, the latest per-verb journal evidence, and the revision-bound confirmation identifier without exposing credentials, provider command lines, or registry bytes.

#### Scenario: Status with no journal rows

- **WHEN** the project has never been preflighted
- **THEN** `forge delivery status` and `GET /v1/projects/{id}/delivery` return `phase=draft`, an empty `evidence` list, and the registered source revision

#### Scenario: Status after a partial run

- **WHEN** the journal carries preflight, stage and promote rows for the same revision
- **THEN** the projection returns `phase=healthy` (or `degraded` if the provider's last `verify` reports unhealthy) plus the latest per-verb evidence, sorted by `op_id` and ordered by verb

#### Scenario: Redaction across surfaces

- Each rendered human table, JSON envelope, UI section, and journal `detail` passes `policy::redact_credentials`; no provider command line, no token-shaped value, no `secret_ref` value, and no query string with a credential key ever appears

