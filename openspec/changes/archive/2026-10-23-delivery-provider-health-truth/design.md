## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The delivery status projection | `forge` | Yes. `src/delivery/projection.rs`. |
| The delivery contract test | `forge` | Yes. `tests/delivery_provider_health_truth_contract.rs` (new). |
| The `delivery-provider-health-truth` capability | `forge` | Added. |
| The `project-to-production-workflow` capability | `forge` | Modified. |
| The delivery handlers | `forge` | Read only. |
| The delivery phase vocabulary | `forge` | Read only. `src/delivery/state.rs`. |
| The doctor's `delivery-state` finding | `forge` | Read only. `src/doctor/delivery.rs`. |
| The delivery promote guards | `forge` | Read only. `src/delivery/handlers.rs`. |
| The Hermora onboarding lane | `forge` | Read only. |
| The delivery API and CLI surfaces | `forge` | Read only. |

## The Gap

### The row carries the answer; the projection does not read it

Every delivery operation row's detail already carries the provider's own
answer:

```json
{
  "evidence": ["…"],
  "recovery": ["…"],
  "provider_status": "succeeded",
  "provider_health": "degraded"
}
```

The projection parses that same detail — for the evidence and recovery
lines — and ignores the health:

```rust
// src/delivery/projection.rs
fn parse_evidence_and_recovery(row: &OperationEntry) -> (Vec<String>, Vec<String>) {
    let detail = row.detail.clone().unwrap_or_default();
    let parsed: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(&detail).unwrap_or_default();
    …
}
```

And the phase is computed from the build status, the run status and the
operation's own state only:

```rust
if promote.op_id.is_some() {
    let degraded = promote.build_status… || promote.run_status… || promote.state == "failed";
    if degraded { return DeliveryPhase::Degraded; }
    return DeliveryPhase::Healthy;
}
```

So the projection had the provider's health in hand and did not use it.

## The Rule

The per-verb report carries the health the row recorded, and the promote
branch of the phase computation reads it.

```rust
// src/delivery/projection.rs
pub struct DeliveryVerbReport {
    …
    /// The health the provider reported for this verb, when it
    /// reported one.
    pub provider_health: Option<String>,
}
```

```rust
let degraded = promote.build_status… || promote.run_status… || promote.state == "failed"
    || promote
        .provider_health
        .as_deref()
        .map(|health| health != "healthy")
        .unwrap_or(false);
```

**Only an explicit answer degrades the phase.** A row whose detail
carries no `provider_health` keeps today's behaviour: the canonical
requirement names a provider that *reports* unhealthy, and a row
without an answer is not an answer of `healthy` either — it is the
absence of one, and the projection must not invent a verdict from it.

**The comparison is exact.** The provider's health is the adapter's own
field; the healthy value is `healthy`, the same string the adapter
contract and the delivery contract use. A provider that answers
`degraded`, `unhealthy` or anything else is not healthy.

**The other branches are untouched.** The `stage` and `preflight`
branches never produced `healthy`, so the provider's health does not
change them. The `hermora` branch is decided by the Hermora adapter's
own state.

## The Contract Test

`tests/delivery_provider_health_truth_contract.rs` drives the real
workflow — preflight, stage, promote — against a stubbed
`forge-publish-provider/0.1.0` adapter on a controlled `PATH`, the same
way `tests/delivery_contract.rs` stubs it, and then reads the status
projection:

- a provider that answers `health: "degraded"`, with a successful
  build and run → the projection reports `phase=degraded`;
- a provider that answers `health: "healthy"` → the projection still
  reports `phase=healthy`.

The stub varies only the health field, so the two cases differ in
exactly the fact under test.

## Failure Boundaries

| Boundary | Behaviour |
|---|---|
| The provider reports no health | The phase keeps today's behaviour; the health is absent, not healthy. |
| The provider reports `healthy` | The phase is `healthy`, as today. |
| The provider reports any other state | The phase is `degraded`. |
| The build or run status is not successful | The phase is `degraded`, as today. |
| The operation's own state is `failed` | The phase is `degraded`, as today. |
| The row's detail is not JSON | The detail parses to an empty map, so the health is absent; the phase keeps today's behaviour. |
| The Hermora adapter answered | The Hermora branch decides the phase, as today. |

## Risks

- **A delivery that was reported healthy is now reported degraded.** For
  a provider that says it is unhealthy that is the point: the canonical
  requirement already demanded it, and the doctor's `delivery-state`
  finding already treats `Degraded` as `FAIL`.
- **A provider that answers something other than `healthy` for its own
  reasons now degrades the phase.** The comparison is exact, so a
  provider that invents a new health value is treated as not healthy.
  That is the conservative reading of "reports unhealthy".
- **The document's shape does not change.** The per-verb report gains no
  new field: the health is read from the detail the row already
  carries, and the projection's response keeps its fields.
