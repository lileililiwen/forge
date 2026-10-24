# Design: publish-fleet-dry-run-verdict-truth

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The one rule | `forge` | Read only. `src/publish/model.rs`. |
| The fleet provider lane | `forge` | Yes. `src/cli/fleet.rs`. |
| The fleet publish contract test | `forge` | Added. `tests/publish_fleet_dry_run_verdict_truth_contract.rs`. |
| The `publish-fleet-dry-run-verdict-truth` capability | `forge` | Added. |
| The `publish-dry-run-verdict-truth` capability | `forge` | Modified. |
| The fleet lane's publish-pipeline path | `forge` | Read only. |
| The fleet exec lane | `forge` | Read only. |
| The publish pipeline lane | `forge` | Read only. |
| `forge publish provider` | `forge` | Read only. |
| The publish contract version | `forge` | Read only. |
| The provider contract | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/cli/fleet.rs, before — the fleet provider lane
Ok(Output::Json(value)) => {
    let healthy = value
        .get("health")
        .and_then(|v| v.as_str())
        .map(|v| v == "healthy")
        .unwrap_or(dry_run);
    render_output(as_output(
        format,
        format!("publish {project_id}: provider={provider_id}"),
        value.clone(),
    ));
    summaries.push(serde_json::json!({
        "project": project_id,
        "provider": provider_id,
        "healthy": healthy,
        "status": value.get("status"),
        "build_status": value.get("build_status"),
        "run_status": value.get("run_status"),
        "container_identity": value.get("container_identity"),
    }));
    let terminal_state = if healthy { "done" } else { "failed" };
    ...
    let container_identity = value
        .get("container_identity")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            phase_revision.map(|rev| {
                forge::publish::providers::compose_project_name(&project_id, rev)
            })
        });
    ...
    if healthy {
        success_count += 1;
    } else {
        failure_count += 1;
        ...
    }
}
```

### What it produced

```
op_id = 1
kind = publish
project_id = alpha
state = done
detail = fleet provider=probe
revision = d593d117fd0dbfede91bca0eee12bae6e3addd6a
container_identity = forge-alpha-d593d117fd0d
```

and a fleet summary with `"healthy": true` and `"success": 1`.

### Why the fallback invents the health

`cmd_publish_provider` answers a dry run with the serialized **request**
(`src/cli/publish.rs:604-608`):

```rust
let response = if dry_run {
    serde_json::to_value(&request)...
} else {
    let response = invoke_provider(&entry, &request, &project_dir)?;
    ...
};
```

`PublishProviderRequest` has no `health` field — the health belongs to
the provider's **response**. So for a dry run the lookup misses and the
`.unwrap_or(dry_run)` fallback answers `true`: the dry-run flag is read
as a health verdict, and `healthy` becomes the claim "the publish is
healthy".

### The private copy of the rule

`let terminal_state = if healthy { "done" } else { "failed" };` is the
fleet provider lane's own copy of "what did this run do". The publish
model already owns the rule (`src/publish/model.rs:291-305`):

```rust
/// The one rule that judges a publish run.
///
/// A run that attempted no write is a plan, whatever its stages say.
/// A run that attempted a write is a delivery only when every stage
/// completed.
pub fn verdict_for_report(dry_run: bool, healthy: bool) -> PublishVerdict {
    if dry_run {
        return PublishVerdict::Planned;
    }
    if healthy {
        PublishVerdict::Delivered
    } else {
        PublishVerdict::Failed
    }
}
```

### The three writers the canonical rule names

| Writer | Where | How it names the run | Rule |
|---|---|---|---|
| The publish pipeline | `src/publish/pipeline.rs` | the report's `verdict` | the model |
| The fleet lane, publish-pipeline path | `src/cli/fleet.rs:683-692` | `report.verdict.journal_state()` | the model |
| The fleet exec lane | `src/cli/fleet_exec.rs:445-457` | `report.verdict.journal_state()` | the model |
| **The fleet lane, provider path** | `src/cli/fleet.rs:548-618` | `if healthy { "done" } else { "failed" }` | **a private copy** |

Three writers read the model; the fourth keeps a copy of its own, and
the copy is wrong in exactly the case the canonical requirement names.

### The canonical rule it breaks

`publish-dry-run-verdict-truth`, requirement *The publish report names
what the run did*:

> A plan SHALL NOT be recorded as a completed publish, and every
> registry journal row for a publish SHALL follow the one rule rather
> than a private copy of it.

> #### Scenario: The fleet lane follows the same rule
> - **WHEN** a fleet publish run is planned, or a fleet project's publish fails
> - **THEN** the per-project state names the run a plan or a failure, and a planned fleet publish is not counted as a failure

> #### Scenario: One rule, every journal writer
> - **WHEN** the publish pipeline, the fleet lane and the fleet exec lane record a publish
> - **THEN** all three read the report's verdict

## The Rule

The provider lane judges its run with the publish model's one rule, and
the health it judges from is the provider's own answer:

```rust
// src/cli/fleet.rs
Ok(Output::Json(value)) => {
    // The provider's answer decides the health. A dry run never
    // contacts the provider, so it has no health to report and the
    // run is a plan, not a delivery.
    let healthy = value
        .get("health")
        .and_then(|v| v.as_str())
        .map(|v| v == "healthy")
        .unwrap_or(false);
    let verdict = forge::publish::verdict_for_report(dry_run, healthy);
    render_output(as_output(
        format,
        format!("publish {project_id}: provider={provider_id}"),
        value.clone(),
    ));
    summaries.push(serde_json::json!({
        "project": project_id,
        "provider": provider_id,
        "healthy": healthy,
        "verdict": verdict.label(),
        "status": value.get("status"),
        "build_status": value.get("build_status"),
        "run_status": value.get("run_status"),
        "container_identity": value.get("container_identity"),
    }));
    let terminal_state = verdict.journal_state();
    ...
    // The container identity is the provider's own. The lane never
    // composes one: a run that published nothing has no container.
    let container_identity = value
        .get("container_identity")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    ...
    let detail = format!(
        "fleet provider={provider_id} verdict={} healthy={}",
        verdict.label(),
        healthy
    );
    if let Ok(op_id) = core_registry.record_queue_publish_phase(
        &project_id, &queue_id, terminal_state, phase, Some(&detail),
    ) {
        queue_op_ids.push((project_id.clone(), op_id));
    }
    if verdict == PublishVerdict::Failed {
        failure_count += 1;
        if fail_fast && first_failure.is_none() {
            first_failure = Some(ForgeError::PublishDeployFailed {
                reason: format!("fleet provider publish failed at `{project_id}`"),
            });
            break;
        }
    } else {
        success_count += 1;
    }
}
```

### Why the model rule, not a new one

The rule already exists and it is already the one the other three
writers read. The defect is not a missing rule; it is a writer that
kept a private copy of it and got the copy wrong. Adding a second rule
for the provider lane would give the publish lane two vocabularies,
which is the defect the requirement forbids.

`verdict_for_report` is named for the report because the report is where
the other writers read it from. The rule itself is the rule for a
publish **run**, and a provider publish run is a publish run: it either
attempted a write or it did not.

### Why the failure count follows the verdict

The canonical scenario says *"a planned fleet publish is not counted as
a failure"*. The fleet lane's publish-pipeline path already does this
(`src/cli/fleet.rs:707-717`): it counts a failure only when the verdict
is `Failed`. The provider lane now counts the same way, so a planned
provider publish is not a failure and does not trip `--fail-fast`.

### The options, decided

| Option for the fleet provider lane | Verdict |
|---|---|
| Keep the `.unwrap_or(dry_run)` fallback | Rejected. The dry-run flag is not a health verdict: it invents `healthy = true` for a run that contacted nobody. |
| Read the health from the provider's answer only, and judge the run with the model's one rule | **Chosen.** The lane reaches the same `PublishVerdict` the other three writers reach, and its health is an observation. |
| Give the provider lane its own `PublishVerdict` rule | Rejected. Two rules for one lane is two vocabularies, which is exactly what the requirement forbids. |
| Make the provider lane build a `PublishReport` and read `report.verdict` | Rejected here. It would satisfy the wording "all three read the report's verdict" most literally, but the provider lane has no publish stages — it hands the request to an external provider and reads one answer. Forcing a report would invent a stage list. The model rule is what the requirement protects: one rule, every writer. |
| Drop the provider branch from the fleet lane | Rejected. The provider branch is the fleet lane's external-provider path; removing it removes a supported surface. |
| Treat a provider answer with no health as healthy | Rejected. "Unverified is not a pass" is the rule the whole product follows; an answer without a health field is not a delivery. |
| Keep the composed container identity for a plan | Rejected. A plan created no container, so the row would name one that does not exist. |

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A dry run of the provider lane | the run is `planned`; the summary's `healthy` is `false` and its `verdict` is `planned`; the journal row's state is `planned`, its detail names the verdict and the health, and it records no container identity; the fleet summary's `success` counts it, because a plan is not a failure. |
| A provider run that reports `health = healthy` | the run is `delivered`; the summary's `healthy` is `true`; the journal row's state is `done`, exactly as before. |
| A provider run that reports any other health | the run is `failed`; the row's state is `failed`, exactly as before. |
| A provider answer with no health field | the run is `failed`; the row's state is `failed`, exactly as before — the fallback's old value was already `false` for a real run. |
| The fleet lane's publish-pipeline path | unchanged: it reads `report.verdict`. |
| The fleet exec lane | unchanged: it reads `report.verdict.journal_state()`. |
| The publish pipeline lane | unchanged. |
| `forge publish provider` | unchanged: a dry run still answers with the serialized request. |
| The fleet summary's shape | unchanged except for the added per-project `verdict` field; the counts, the keys and the contract version stay as they are. |
| The exit code | unchanged: a plan is not a failure, so the command exits 0. |
| The provider contract | unchanged. |
| The publish contract version | unchanged. |

## Risks

| Risk | Mitigation |
|---|---|
| The provider lane's rows change state for a dry run. | The value was false: a planned publish was recorded as a completed publish, which the canonical requirement already forbids. |
| A reader counts `state = done` publish rows to count deliveries. | That count was wrong before this change: it counted dry runs as deliveries. |
| The provider lane's per-project row gains a `verdict` field. | It is additive, and the fleet lane's publish-pipeline row already carries the same field. |
| The real provider path loses the container identity. | It does not: `cmd_publish_provider` already gives the response the conventional identity when the provider reports none (`src/cli/publish.rs:615-617`), so the lane reads the same value it read before. |
| A provider that answers without a health field now fails. | It already failed: the old fallback was `false` for a real run. |
| The rule's name says "report" while the provider lane has no report. | The rule's own doc says it judges a publish run; the name records where the other writers read it from. A rename would touch change 33's published surface for no behaviour change, so it is not done here. |
