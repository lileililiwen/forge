# Design: publish-container-identity-evidence

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The publish identity rule | `forge` | Added. `src/publish/providers/mod.rs`. |
| The CLI publish lane | `forge` | Yes. `src/cli/publish.rs`. |
| The sibling-cwd publish lane | `forge` | Yes. `src/cli/publish.rs`. |
| The operator publish route | `forge` | Yes. `src/api/handlers_core.rs`. |
| The admin publish route | `forge` | Yes. `src/api/admin/publish.rs`. |
| The publish contract test | `forge` | Added. `tests/publish_container_identity_evidence_contract.rs`. |
| The `forge-publish-observability-revision-containers` capability | `forge` | Modified. |
| The fleet lane, the delivery lane, the projection | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/cli/publish.rs, before
let container_identity = response.container_identity.clone().unwrap_or_else(|| {
    forge::publish::providers::compose_project_name(project_id, &phase_revision)
});
```

### What it produced

```
failed|forge-demo-0123456789ab
```

The row's state says the publish failed; the row names a container for
it.

### The evidence, and where it is not

| The fact | Where it lives |
|---|---|
| The publish failed | The row's own `state` column, which is the provider's own status. |
| The revision that was attempted | The row's `revision` column. |
| A container that was deployed | Nothing. The provider reported no identity and the publish did not complete. |

### The rule that already exists

The canonical capability draws the line:

> **Scenario: Provider without container_identity synthesizes it from
> revision** — **WHEN** a provider returns a **successful** response
> without a `container_identity` field — **THEN** Forge persists the row
> with the canonical `forge-<project>-<sha12>` identity computed from the
> recorded revision so legacy providers are not silently absent from
> observability.

The provider contract's field doc says the same:

```rust
// src/publish/providers/model.rs
/// Bounded container / Compose project identity the provider
/// actually deployed ... absent when the provider cannot prove the
/// identity.
pub container_identity: Option<String>,
```

And one lane already reads it:

```rust
// src/cli/fleet.rs
// The container identity is the provider's own.
// The lane never composes one: a run that published nothing has no container.
```

So the rule was written down twice and implemented in one lane. The
manual lanes are the ones that claim a container nobody reported.

## The Rule

**One rule names the container identity a response proves: the provider's
own identity is the truth, the canonical identity is composed only for a
response that published, and a response that published nothing carries
no identity at all.**

### The rule lives in the provider contract

```rust
// src/publish/providers/mod.rs, after
/// The provider terminal statuses that say the publish completed.
///
/// The lanes already read two words as a completed run: the publish
/// contract's `done` and the delivery lane's `succeeded`. Both mean
/// the provider published.
pub const PUBLISH_COMPLETED_STATUSES: [&str; 2] = ["done", "succeeded"];

/// The container identity a publish response proves.
///
/// The provider's own identity is the truth and is always kept. A
/// legacy provider that published but reported no identity gets the
/// canonical `forge-<project>-<sha12>` identity composed from the
/// recorded revision, so it is not silently absent from observability.
/// A response that published nothing carries no identity at all: Forge
/// does not invent a container for a deployment that never happened.
pub fn proven_container_identity(
    response: &PublishProviderResponse,
    project_id: &str,
    recorded_revision: &str,
) -> Option<String> {
    if let Some(identity) = response.container_identity.as_deref() {
        return Some(identity.to_string());
    }
    if !PUBLISH_COMPLETED_STATUSES.contains(&response.status.as_str()) {
        return None;
    }
    Some(compose_project_name(project_id, recorded_revision))
}
```

### Every publish surface reads it

```rust
// src/cli/publish.rs, after
let container_identity = proven_container_identity(&response, project_id, &phase_revision);
...
PublishPhaseEvidence::new()
    .revision(&phase_revision)
    .container_identity_opt(container_identity.as_deref())
    .build_status_opt(response.build_status.as_deref())
    .run_status_opt(response.run_status.as_deref()),
```

The operator route and the admin route read the same function, and
their responses carry the same value the row carries.

## The options, decided

| Option for a response that published nothing | Verdict |
|---|---|
| Keep composing the canonical identity | Rejected. The row then names a container for a deployment that never happened, and contradicts its own state. |
| Never compose, in any lane | Rejected. The canonical capability authorises the synthesis for a successful response so a legacy provider is not silently absent from observability. The existing contract test pins that. |
| Compose only for a response that published | **Chosen.** It is what the canonical scenario says, what the provider contract's field doc says, and what the fleet lane already does. |
| Compose only when the phase evidence says `run_status=succeeded` | Rejected. It would ignore the provider's own terminal status and change the existing legacy-provider contract, whose provider reports no phase evidence at all yet published. |
| Drop the row's identity column for a failure | Rejected. The row still carries the revision and the phase evidence; only the identity is absent, and the projection renders it as `unknown`. |

| Option for where the rule lives | Verdict |
|---|---|
| A helper per lane | Rejected. Four copies of one rule is how the lanes drifted apart in the first place. |
| One function in the provider contract, read by every lane | **Chosen.** The provider contract owns the runtime identity ("Owns the revision-bound phase evidence and the runtime identity the contract guarantees"), so the rule belongs there and every surface reads it. |
| A rule in the registry | Rejected. The registry stores the row; it does not know what the provider published. |

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A successful response with an identity | The provider's identity, unchanged. |
| A successful response without an identity | The canonical `forge-<project>-<sha12>` identity, unchanged. |
| A failed response with an identity | The provider's identity — the provider's own report is the truth. |
| A failed response without an identity | No identity. The row and the response carry none, and the projection renders `unknown`. |
| A dry run | Unchanged: no row, no identity. |
| A provider the harness cannot reach | Unchanged: the publish fails before a row is written. |
| The fleet lane | Unchanged: it keeps only the provider's own identity, which is what the rule says. |
| The delivery lane | Unchanged: it writes no identity at all. |

## Risks

| Risk | Mitigation |
|---|---|
| A legacy provider that published but reported a status the rule does not know loses its identity. | The rule keeps the provider's own identity in every case. Only the composition is limited, and the composition exists so a legacy provider is not absent. The two completion words are the ones the lanes already read (`done` in the publish contract, `succeeded` in the delivery lane). |
| A reader counted publish rows with a container identity to find running deployments. | That count was wrong for a failed publish: it counted a container that was never deployed. The new rows say which deployments happened. |
| The response's `container_identity` field changes shape for a failed publish. | It becomes `null`, which is the honest value; the projection already renders a missing value as `unknown` and the canonical capability says missing values render as `unknown`, never as a verified success. |
