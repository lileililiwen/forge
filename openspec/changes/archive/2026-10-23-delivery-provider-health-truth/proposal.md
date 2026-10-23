# Change: delivery-provider-health-truth

## Why

`forge delivery status` projects the project's delivery `phase` from the
per-verb journal rows. The canonical capability says what that phase
must be:

> #### Scenario: Status after a partial run
>
> - **WHEN** the journal carries preflight, stage and promote rows for
>   the same revision
> - **THEN** the projection returns `phase=healthy` (or `degraded` if
>   the provider's last `verify` reports unhealthy) plus the latest
>   per-verb evidence, sorted by `op_id` and ordered by verb

The projection never reads the provider's health:

```rust
// src/delivery/projection.rs
if promote.op_id.is_some() {
    let degraded = promote
        .build_status
        .as_deref()
        .map(|s| s != "succeeded" && s != "not_started" && s != "unknown")
        .unwrap_or(false)
        || promote
            .run_status
            .as_deref()
            .map(|s| s != "succeeded" && s != "not_started" && s != "unknown")
            .unwrap_or(false)
        || promote.state.as_deref() == Some("failed");
    if degraded {
        return DeliveryPhase::Degraded;
    }
    return DeliveryPhase::Healthy;
}
```

Only the build status, the run status and the operation's own state are
read. The provider's own answer is recorded — the delivery handler
writes it into every operation row's detail — and the projection parses
that same detail for the evidence and recovery lines, but never for the
health:

```rust
// src/delivery/handlers.rs
serde_json::to_string(&json!({
    "evidence": evidence,
    "recovery": recovery,
    "provider_status": response.status,
    "provider_health": response.health,
}))
```

**A provider that says it is unhealthy is projected as healthy.**
Driven on this host with a stubbed `forge-publish-provider/0.1.0`
adapter — the same stubbing pattern `tests/delivery_contract.rs` uses —
that answered `health: "degraded"` with `build_status: "succeeded"`,
`run_status: "succeeded"` and `status: "succeeded"`, the projection
answered:

```json
{"phase":"healthy","environment":"production", …}
```

The provider had said it was degraded. The phase said healthy.

**The phase is the only place the verdict is stated.** The projection's
per-verb evidence carries the provider's own lines (`"provider stub"`
above) but not its health, so nothing in the document named the
contradiction.

**The rest of the platform already trusts the provider's answer.** The
doctor's `delivery-state` finding maps `DeliveryPhase::Degraded` to
`FAIL` (`src/doctor/delivery.rs`), and the delivery lane's own promote
guards refuse to promote a stage that is not healthy. The status
projection was the one surface that discarded it.

## What Changes

- **The projection reads the provider's own health.** The promote
  branch of the phase computation treats a provider that reported
  anything other than `healthy` as `degraded`, exactly as the canonical
  requirement says.
- **The provider's health reaches the projection.** The per-verb report
  carries the health the operation row recorded, so the projection stops
  filtering the row for a field it ignores.
- **The claims are pinned.** A repository contract test drives the real
  delivery workflow against a stubbed provider and holds the phase to
  the provider's answer.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The delivery status projection | `forge` | Yes | `src/delivery/projection.rs`: the phase it computes and the per-verb report it reads. |
| The delivery contract test | `forge` | Yes | `tests/delivery_provider_health_truth_contract.rs` (new). |
| The `delivery-provider-health-truth` capability | `forge` | Added | this package |
| The `project-to-production-workflow` capability | `forge` | Modified | The read-only status requirement names the rule. |
| The delivery handlers | `forge` | Read only | They already record `provider_health` in every operation row's detail. |
| The delivery phase vocabulary | `forge` | Read only | `DeliveryPhase::Degraded` already exists and is already rendered as `degraded`. |
| The doctor's `delivery-state` finding | `forge` | Read only | It already maps `Degraded` to `FAIL`; a degraded delivery now reaches it. |
| The delivery promote guards | `forge` | Read only | Unchanged. |
| The Hermora onboarding lane | `forge` | Read only | Unchanged. |
| The API and CLI delivery surfaces | `forge` | Read only | They serialise the projection; no field changes. |
| The delivery contract version | `forge` | Read only | No field is added, removed or renamed. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §27 (distribution) and §30 (deployment) own the staged delivery; the delivery capability owns the status projection. |
| Contracts | The projection's `phase` changes for one class of project — a promote whose provider reported an unhealthy state — from `healthy` to `degraded`. The per-verb report gains no new field: the health is read from the detail the row already carries. No field is added, removed or renamed, and the contract version is unchanged. |
| Callers | `forge delivery status`, `GET /v1/projects/{id}/delivery`, the portal's delivery section, the doctor's `delivery-state` finding. |
| Persistence | Nothing new is persisted. The provider's health is already in the operation row's detail. |
| Integrations | None. The adapter is invoked exactly as before. |
| Failures | No new error code. |
| Tests | A new `tests/delivery_provider_health_truth_contract.rs`. |
| Dependencies | None added. |
| Concerns | Quality: the phase states the provider's own answer. Compatibility: the document's shape is unchanged; one classification becomes more precise, and the canonical requirement already demanded it. Security: the health string is the provider's own field, bounded to the same shape the adapter already returned; no credential surface is added. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `delivery-provider-health-truth` | forge | Rust | added | this package |
| `project-to-production-workflow` | forge | Rust | modified | the read-only status requirement names the rule |

## Non-Goals

- **The delivery workflow is unchanged.** Preflight, stage, promote and
  the Hermora onboarding behave exactly as before.
- **The promote guards are unchanged.** A stage that is not healthy is
  still refused.
- **The phase vocabulary is unchanged.** `degraded` already existed and
  is already rendered as `degraded`.
- **A provider that reported no health is not degraded.** The canonical
  requirement names a provider that *reports* unhealthy; a row without
  a health keeps today's behaviour.
- **The stage and preflight branches are unchanged.** They never
  produced `healthy`, so the provider's health does not change them.
- **The delivery contract version is unchanged.**
