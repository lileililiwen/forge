## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The healthy-release rule and the rollback projection | `forge` | Yes. `src/delivery/projection.rs`. |
| The rollback verb, its target selector and its idempotency key | `forge` | Yes. `src/delivery/{state,handlers}.rs`. |
| The provider request field | `forge` | Yes. `src/publish/providers/model.rs`, `src/delivery/invoke.rs`. |
| The CLI subcommand | `forge` | Yes. `src/cli/`. |
| The doctor's reading of the phase | `forge` | Yes. `src/doctor/delivery.rs`. |
| The provider contract document | `forge` | Yes. `docs/adapter-contracts/publish-provider.md`. |
| The provider binaries | their owners | No. A provider that declares `rollback` must accept `target_revision`. |
| The `operations` schema | `forge` | No. One more `kind` value, same columns. |
| The API and web routes | `forge` | No. No new route; read-only projections pick up the field. |
| The publish lane's Mac-script rollback | `forge` | No. That is a different mechanism with its own spec. |

## One Healthy-Release Rule

`compute_phase` already decides whether the latest promotion is healthy:

- the row is not `failed`;
- `build_status` and `run_status` are not a value other than
  `succeeded`/`not_started`/`unknown`;
- an explicit `provider_health` is `healthy`.

Rollback must use exactly that rule, or Forge would restore a release the
status page calls degraded. The rule moves into one function over
`OperationEntry`:

```rust
pub fn promotion_is_healthy(row: &OperationEntry) -> bool;
pub fn healthy_release_revisions(rows: &[OperationEntry]) -> Vec<String>;
```

`compute_phase` keeps its shape but calls the same predicate for the latest
promote row, so the two readers cannot drift. `healthy_release_revisions`
walks the promotion rows newest first, keeps the newest row per revision, and
returns the healthy revisions in promotion order.

A revision that was promoted twice — once failed, once healthy — counts as a
release because the healthy row exists. A revision whose newest promotion is
degraded does not count, even if an older row for it was healthy: the later
row is what production last observed.

## Target Selection

The CLI accepts exactly one selector:

| Invocation | Resolution |
|---|---|
| `--to <revision>` | that revision, after the healthy-release check |
| `--previous` | the newest healthy release whose revision differs from the registered one |

Both `--to` and `--previous` is a `delivery-invalid` refusal; neither is the
same refusal. The registered revision is never a valid target: rolling back to
the release already in production is not a rollback, and the sentence says so.

The resolved target is recorded in the request (`target_revision`), in the
journal detail, and in the status report. `--previous` is therefore auditable:
the operator asked for "the previous one", and the row names which one that
was when the verb ran.

## Idempotency

The existing key is `(verb, project, revision)`. That is wrong for rollback,
because two rollbacks from the same broken revision to two different good
revisions are two operations, not a conflict. Rollback gets its own key:

```text
delivery.rollback.<project>:<current-sha12>:<target-sha12>
```

and the request hash uses the target as its confirmation token. A replay of
the same `(current, target)` reuses the row; a different target reserves a new
one.

## The Request

`PublishProviderRequest` gains one optional field:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub target_revision: Option<String>,
```

`revision` keeps its meaning: the revision the request operates on, which for
rollback is the registered revision being replaced. `target_revision` is the
release being restored. The field is present only on `rollback`; every other
operation serialises the same bytes it did before.

The contract version does not move. `rollback` was declared in `0.1.0` and
never sent, so the first use defines its parameter and no deployed provider can
depend on the old shape. The contract document states the rule: a provider
that declares `rollback` MUST read `target_revision`.

## Refusals

| Situation | Code | Sentence |
|---|---|---|
| both or neither selector | `delivery-invalid` | names the two accepted selectors |
| `--to` is not 40-hex | `delivery-invalid` | names the value |
| target equals the registered revision | `delivery-conflict` | "already the registered revision" |
| target has no healthy promotion | `delivery-conflict` | names the revision and says only a promoted, healthy release can be restored |
| `--previous` with no earlier healthy release | `delivery-conflict` | names the project and the condition |
| provider does not declare `rollback` | `delivery-invalid` | the capability sentence from the previous change |
| provider fails | `delivery-unavailable` or the provider's typed error | unchanged |

All of them happen before `reserve_idempotent_operation`, except the provider
failure, which finalises a failed row.

## Phase and Status

`DeliveryPhase` gains `RolledBack` (`rolled-back`). `compute_phase` evaluates
it first: a latest terminal `delivery.rollback` row in state `done` means
production was restored. A failed rollback row does not change the phase — the
release in production did not move — and the row itself records the failure.

`DeliveryReport` gains `rollback: DeliveryVerbReport`, and
`DeliveryVerbReport` gains `target_revision: Option<String>` (skipped when
absent). The status contract moves to `forge-delivery-status/0.2.0`: an added
verb and phase are additive to readers but change the document's vocabulary.

The doctor maps `rolled-back` to `WARN` with a reason that says production
serves an earlier release and the registered revision is not the live one. It
is not a failure: the operator chose it. It is not a pass either, because the
project is not on the revision it is registered at.

## Verification

- unit tests for the healthy-release rule, target resolution, the key and the
  request field;
- CLI contract tests with a fixture provider that declares `rollback`, a
  second that does not, an unknown provider, an unpromoted target, `--previous`
  with and without an earlier release, a replay, and a provider failure;
- the existing delivery, doctor, API and web suites as the regression oracle;
- the repository suite and the repository Gate.
