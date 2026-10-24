# Change: publish-fleet-dry-run-verdict-truth

## Why

The fleet publish's **provider** lane records a **completed publish** in
the registry journal for a run that published nothing.

Reproduced on the current tree, with an inventory whose one entry is
compose-ready and a project that is a git worktree at a real revision:

```
$ forge publish fleet --inventory /tmp/pub-probe/inventory.json \
    --provider probe --dry-run
```

The fleet summary says:

```json
"projects": [
  {
    "build_status": null,
    "container_identity": null,
    "healthy": true,
    "project": "alpha",
    "provider": "probe",
    "run_status": null,
    "status": null
  }
],
"success": 1
```

and the append-only row the same run wrote:

```
op_id = 1
kind = publish
project_id = alpha
state = done
detail = fleet provider=probe
revision = d593d117fd0dbfede91bca0eee12bae6e3addd6a
container_identity = forge-alpha-d593d117fd0d
```

`state='done'`, `healthy=true`, `success=1`, and a container identity
for a container that was never created. **No provider ran**: the dry run
returns the serialized *request*, which has no health field at all.

**The lane already knows the difference.** The fleet lane's other path —
the publish pipeline — reads the report's verdict
(`src/cli/fleet.rs:683-692`) and records `planned` for a plan. The
provider lane kept a private copy of the rule:

```rust
// src/cli/fleet.rs, before
let healthy = value
    .get("health")
    .and_then(|v| v.as_str())
    .map(|v| v == "healthy")
    .unwrap_or(dry_run);              // ← a dry run invents `true`
...
let terminal_state = if healthy { "done" } else { "failed" };
```

The `.unwrap_or(dry_run)` fallback is the defect: when the provider's
answer carries no health — which is always the case for a dry run,
because there is no provider answer — the lane reads the dry-run flag as
a health verdict.

**The canonical rule already forbids this.** The `publish-dry-run-
verdict-truth` capability says:

> A plan SHALL NOT be recorded as a completed publish, and every registry
> journal row for a publish SHALL follow the one rule rather than a
> private copy of it.

> #### Scenario: The fleet lane follows the same rule
> - **WHEN** a fleet publish run is planned, or a fleet project's publish fails
> - **THEN** the per-project state names the run a plan or a failure, and a planned fleet publish is not counted as a failure

> #### Scenario: One rule, every journal writer
> - **WHEN** the publish pipeline, the fleet lane and the fleet exec lane record a publish
> - **THEN** all three read the report's verdict

The provider lane breaks both. This is the rule the repair programme is
built on: Forge states something its own gathered evidence does not
support.

## What Changes

- **The fleet provider lane reads the one rule.** The publish model's
  `verdict_for_report` judges the run: a dry run is a plan, a run that
  contacted the provider is a delivery only when the provider reported
  the target healthy.
- **The lane's health comes from the provider's answer.** An answer with
  no health field is not healthy; the dry-run flag is not a health
  verdict.
- **The row names the run and claims no container.** The journal detail
  carries `verdict=` and `healthy=`, and a planned publish records no
  container identity, because none was created.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The one rule | `forge` | Read only | `src/publish/model.rs`: `PublishVerdict` and `verdict_for_report` are used as they are. |
| The fleet provider lane | `forge` | Yes | `src/cli/fleet.rs`: the health, the verdict, the journal state, the detail and the phase evidence. |
| The fleet publish contract test | `forge` | Added | `tests/publish_fleet_dry_run_verdict_truth_contract.rs`. |
| The `publish-fleet-dry-run-verdict-truth` capability | `forge` | Added | this package |
| The `publish-dry-run-verdict-truth` capability | `forge` | Modified | its fleet scenarios name the provider lane. |
| The fleet lane's publish-pipeline path | `forge` | Read only | It already reads the report's verdict. |
| The fleet exec lane | `forge` | Read only | It already reads `report.verdict.journal_state()`. |
| The publish pipeline lane | `forge` | Read only | Unchanged. |
| `forge publish provider` | `forge` | Read only | It answers the provider's own request; the fleet lane judges it. |
| The publish contract version | `forge` | Read only | Unchanged. |
| The provider contract | `forge` | Read only | Unchanged. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §30 owns the publish pipeline and its fleet lane; the `publish-dry-run-verdict-truth` capability owns the publish verdict rule and already requires every publish journal row to follow it. |
| Contracts | No field is removed or renamed. `PUBLISH_CONTRACT_VERSION` and `PUBLISH_PROVIDER_CONTRACT` are unchanged, the provider request and response are unchanged, and the fleet summary's per-project row gains one `verdict` field. |
| Callers | `forge publish fleet --provider <id>`, its fleet summary and its registry journal rows. |
| Persistence | The journal row's state for a planned provider publish becomes `planned`, its detail gains the verdict and the health, and it no longer records a container identity. |
| Integrations | None. The provider contract, the provider adapter and the provider's own request are untouched; the lane only reads the provider's answer. |
| Failures | No new error code. A planned publish is not a failure, so the command still exits 0; a provider that reports no health is now a failure, as it already was. |
| Tests | A new contract test `tests/publish_fleet_dry_run_verdict_truth_contract.rs`; the existing publish, fleet and inventory tests stay green. |
| Dependencies | None added. |
| Concerns | Quality: the fleet provider lane can no longer be read as "the publish happened" when only a request was sent, and it claims no container for a plan. Compatibility: a reader of the provider lane's rows sees `planned` instead of `done` for a dry run, the per-project row gains a `verdict` field, and the row no longer carries a composed container identity for a plan. Security: no new surface; the change removes a written identity rather than adding one. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `publish-fleet-dry-run-verdict-truth` | forge | Rust | added | this package |
| `publish-dry-run-verdict-truth` | forge | Rust | modified | its fleet scenarios name the provider lane. |

## Non-goals

- **The provider contract is unchanged.** The request and the response
  keep their fields; a dry run still answers with the serialized
  request.
- **`forge publish provider` is unchanged.** It answers the provider's
  own request; the fleet lane judges the answer.
- **The publish pipeline's verdict is unchanged.** The lane already
  reads the report's verdict.
- **The fleet exec lane is unchanged.** It already reads
  `report.verdict.journal_state()`.
- **The publish contract version is unchanged.**
- **The provider's own default container identity is unchanged.** When a
  provider really runs and reports no container identity, the provider
  lane still records the conventional name. This change only stops the
  fleet lane from composing one for a run that never happened.
