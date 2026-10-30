# Change: delivery-rollback

## Why

The report that prompted this programme named rollback as the one first-class
delivery verb Forge was missing, and the repository agrees with it. The
delivery lane records `preflight`, `stage`, `promote` and the optional Hermora
enrollment. There is no way to put an earlier release back into production
through Forge, even though every promotion is recorded, revision-bound and
carries the provider's own health.

The two pieces that make rollback safe now exist. The provider contract
declares a `rollback` operation, and the previous change made the provider's
declaration reachable, so Forge can ask whether a provider serves `rollback`
before it sends one. What is still missing is the thing rollback needs most: a
release record that answers "which revisions were promoted and observed
healthy". The status projection cannot answer it — it takes the latest row per
verb, so it reports one revision per verb and mixes revisions across verbs.

`forge delivery status` therefore says `healthy` about the registered
revision, and there is no verb that acts on the history behind it. An operator
whose production is broken has to re-register an old commit, re-run Gate, and
promote again — which produces a promotion that looks like a new release
rather than a restoration of a known-good one.

## What Changes

- **`forge delivery rollback` becomes a verb.** It takes exactly one target
  selector: `--to <40-hex revision>` or `--previous`. The target must be a
  revision Forge promoted and observed healthy; anything else is refused with
  `delivery-conflict` before the provider is resolved.
- **A healthy release is read from the promotion rows, once.** One shared
  function decides whether a `delivery.promote` row records a healthy release,
  and both `compute_phase` and rollback call it. Rollback never trusts the
  mixed per-verb projection.
- **The provider request gains `target_revision`.** `revision` stays the
  project's registered revision — the release being replaced — and
  `target_revision` is the release being restored. The field is additive:
  `rollback` was declared and unreachable, so no provider has ever received a
  rollback request under the old shape.
- **The provider must declare `rollback`.** An unknown declaration, or one
  that omits `rollback`, is refused before the operation is reserved. This is
  the first verb the previous change's gate protects.
- **Rollback does not re-run the ship gate.** It restores a release whose
  promotion is already recorded healthy; requiring fresh Gate evidence for the
  *broken* revision would invert the point of rolling back. The healthy
  promotion row is the evidence, and it is checked first.
- **`forge delivery status` gains a `rollback` verb and a `rolled-back`
  phase.** The status contract moves to `forge-delivery-status/0.2.0`; the
  rollback report carries `target_revision`, and a failed rollback records
  `state=failed` without claiming the release moved.
- **The doctor reads the new phase.** `rolled-back` is a warning that names the
  state: production serves an earlier release, and the registered revision is
  not the live one.

## BFS Impact Map

| Surface | Effect |
|---|---|
| `src/delivery/projection.rs` | `DeliveryReport.rollback`, `DeliveryVerbReport.target_revision`, `RolledBack`, the shared healthy-release rule, the human table |
| `src/delivery/state.rs` | `DeliveryVerb::Rollback`, `RollbackTarget`, the target-scoped idempotency key |
| `src/delivery/invoke.rs` | rollback queue id, `operation_for`, the rollback request with `target_revision` |
| `src/delivery/handlers.rs` | `run_rollback`, target resolution, the healthy-release proof |
| `src/delivery/mod.rs` | the status contract version |
| `src/publish/providers/model.rs` | `PublishProviderRequest.target_revision` |
| `src/cli/commands.rs`, `src/cli/delivery.rs`, `src/cli/delivery_*` | the `rollback` subcommand and its two selectors |
| `src/doctor/delivery.rs` | the `rolled-back` finding |
| `src/api/model.rs`, `src/api/admin/*` | read-only projections pick up the new verb and phase; no new route |
| `docs/adapter-contracts/publish-provider.md` | the rollback operation and `target_revision` |
| `docs/ci-cd-system-model.md` | the flow and the state machine |
| Existing status tests | the contract version moves to `0.2.0` |

Persistence: no schema change and no new table. The rollback is one more
`operations` row with `kind=delivery.rollback`; the target rides in `detail`
and in the request.

Failure boundaries: an unproven target, a current-revision target, a provider
that does not declare `rollback`, and a bad selector all refuse before
reservation. A provider failure records a failed row and leaves the phase
where it was.

## Capabilities

| Capability | State | Notes |
|---|---|---|
| `delivery-rollback` | added | this package |
| `delivery-provider-capabilities` | read only | its gate is the precondition |
| `project-to-production-workflow` | read only | its requirements remain true; rollback is an additional verb |
| `doctor-maturity-assessment` | read only | consumes the new phase |
| `decoupled-remote-publish` | read only | its "one-cycle rollback" is a Mac-script rollback, not this verb |

## Non-Goals

- No automatic rollback. A provider's unhealthy answer never starts a
  rollback; the operator invokes it.
- No API or web route. The CLI is the operational surface for this change; an
  authenticated route is a later change with its own CSRF and preview rules.
- No traffic split, canary or blue-green. Rollback restores one release.
- No release table. The promotion rows are the release history; this change
  reads them and adds no persistence.
- No Gate evidence requirement for the target release. Its healthy promotion
  is the proof; the ship gate continues to guard new promotions only.
- No change to the publish-provider contract version. `target_revision` is
  additive and only reachable through an operation that was never sent.
