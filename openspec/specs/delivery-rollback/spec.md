# delivery-rollback Specification

## Purpose
The delivery lane could move a project forward and never back. It recorded
every promotion with the provider's own health, but nothing read that history:
the status projection reports the latest row per verb, so it cannot answer
"which releases were promoted and observed healthy". This capability adds
`forge delivery rollback`, which restores a release Forge already promoted and
observed healthy — selected explicitly with `--to` or `--previous`, refused for
any revision without that proof, gated by the provider's declared `rollback`
capability, and recorded as its own operation with the restored revision in
the status document's new `rolled-back` phase.
## Requirements
### Requirement: Rollback restores a previously promoted healthy release

Forge SHALL provide a delivery rollback verb that restores a release the
project previously promoted and observed healthy. The operator SHALL select
exactly one target: a 40-character revision with `--to`, or `--previous` for
the newest healthy release whose revision differs from the project's registered
revision. Forge SHALL treat a `delivery.promote` row as a healthy release only
when the row is terminal in state `done`, its `build_status` and `run_status`
are not a value other than `succeeded`, `not_started` or `unknown`, and an
explicit provider health is `healthy` — the same rule the delivery phase uses.
Forge SHALL refuse a target that has no healthy promotion, the registered
revision itself, and a `--previous` request with no earlier healthy release,
with `delivery-conflict` before the provider is resolved.

#### Scenario: A promoted healthy release is restored

- **WHEN** the operator rolls back with `--to` naming a revision whose newest
  promotion row is healthy
- **THEN** Forge invokes the provider's `rollback` operation for that target
  and records the restoration

#### Scenario: The target was never promoted

- **WHEN** `--to` names a revision with no `delivery.promote` row for the
  project
- **THEN** Forge refuses with `delivery-conflict`, names the revision, and
  invokes no provider

#### Scenario: The target's newest promotion was degraded

- **WHEN** the target revision's newest promotion recorded a failed build, a
  failed run, or a provider health other than `healthy`
- **THEN** Forge refuses the rollback instead of restoring a release it
  observed as degraded

#### Scenario: The previous release is requested

- **WHEN** the operator selects `--previous`
- **THEN** Forge resolves the newest healthy release whose revision differs
  from the registered one, and records that resolved revision as the target

#### Scenario: There is no earlier release

- **WHEN** `--previous` is requested and the project has no healthy release
  other than the registered revision
- **THEN** Forge refuses with `delivery-conflict` and names the condition

#### Scenario: The registered revision is named

- **WHEN** `--to` names the revision the project is already registered at
- **THEN** Forge refuses with `delivery-conflict` and says it is already the
  registered revision

### Requirement: Rollback is declared, selected and recorded

Forge SHALL refuse a rollback when the provider's capability declaration does
not contain `rollback`, before the delivery operation is reserved. A rollback
request SHALL carry the project's registered revision as `revision` and the
restored revision as `target_revision`, and SHALL be recorded as one
`delivery.rollback` operation keyed by the project, the current revision and
the target revision. A replay of the same pair SHALL reuse the row; a different
target SHALL reserve a new one. Rollback SHALL NOT require Gate evidence for
the current revision, because it restores a release whose healthy promotion is
its evidence.

#### Scenario: The provider declares rollback

- **WHEN** the provider's declaration contains `rollback`
- **THEN** Forge sends `operation: rollback` with `revision` set to the
  registered revision and `target_revision` set to the restored revision

#### Scenario: The provider does not declare rollback

- **WHEN** the provider declares a set without `rollback`, or makes no readable
  claim
- **THEN** Forge refuses with `delivery-invalid`, names the provider, the
  required operation and the declaration, and reserves no operation

#### Scenario: The rollback is replayed

- **WHEN** the same project, current revision and target revision are requested
  again
- **THEN** Forge reuses the recorded rollback operation instead of invoking the
  provider a second time

#### Scenario: A different target is requested

- **WHEN** the same current revision is rolled back to a different target
- **THEN** Forge reserves a new rollback operation rather than reporting an
  idempotency conflict

#### Scenario: Gate evidence is absent

- **WHEN** the current revision has no fresh passing Gate evidence and a
  healthy target exists
- **THEN** the rollback still proceeds, because the ship gate guards new
  promotions and not the restoration of an already-promoted release

### Requirement: A rollback is observable in delivery status

`forge delivery status` SHALL report a `rollback` verb beside `preflight`,
`stage` and `promote`, carrying the resolved target revision. The delivery
phase SHALL be `rolled-back` when the latest terminal rollback succeeded, and a
failed rollback SHALL record `state=failed` without changing the phase. The
status document SHALL move to `forge-delivery-status/0.2.0`. The doctor SHALL
report `rolled-back` as a warning that names the state.

#### Scenario: A successful rollback is visible

- **WHEN** a rollback completes successfully
- **THEN** the status reports phase `rolled-back`, a `rollback` row in state
  `done`, and the target revision, and the doctor reports a warning naming the
  state

#### Scenario: A failed rollback does not claim the release moved

- **WHEN** the provider fails the rollback operation
- **THEN** the status records the rollback row in state `failed` and keeps the
  phase the project had before the attempt

#### Scenario: The status vocabulary is versioned

- **WHEN** a consumer reads the delivery status document
- **THEN** it sees `forge-delivery-status/0.2.0`, the rollback verb and the
  `rolled-back` phase, and every field the `0.1.0` document published

