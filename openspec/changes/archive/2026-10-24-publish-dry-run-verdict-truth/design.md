## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The verdict rule | `forge` | Yes. `src/publish/model.rs`. |
| The publish pipeline | `forge` | Yes. `src/publish/pipeline.rs`. |
| The publish CLI | `forge` | Yes. `src/cli/publish.rs`. |
| The fleet exec lane | `forge` | Yes. `src/cli/fleet_exec.rs`. |
| The fleet lane | `forge` | Yes. `src/cli/fleet.rs`. |
| The publish contract test | `forge` | Added. `tests/publish_dry_run_verdict_truth_contract.rs`. |
| The `publish-dry-run-verdict-truth` capability | `forge` | Added. |
| The `decoupled-remote-publish` capability | `forge` | Modified. |
| The release lane's publish stage | `forge` | Read only. |
| The MCP publish method | `forge` | Read only. |
| The deploy lane | `forge` | Read only. |
| The publish contract version | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/publish/pipeline.rs, before
let verdict = if request.dry_run || healthy {
    "done"
} else {
    "failed"
};
let _ = reg.record_operation(
    "publish",
    &request.project_id,
    verdict,
    &format!(
        "publish {} summary via {}: healthy={} stages={}",
        request.action.label(),
        adapter.id(),
        healthy,
        stages.len()
    ),
);
```

The row says `done` and its detail says `healthy=true` for a run that
published nothing.

### The same document says the opposite

```rust
// src/publish/pipeline.rs, before
let note = if request.dry_run {
    format!("dry-run: {} stage(s) planned, no side effects", stages.len())
} else if healthy {
```

### The rule, written three times

| Where | The expression |
|---|---|
| `src/publish/pipeline.rs` | `if request.dry_run \|\| healthy { "done" }` |
| `src/cli/fleet_exec.rs` | `if report.dry_run \|\| report.healthy { "done" }` |
| `src/cli/fleet.rs` | `if healthy { "done" } else { "failed" }`, where `healthy` comes from the report's boolean |

Three copies of one decision. None of them can say "planned".

### The report's boolean

```rust
// src/publish/model.rs, before
pub struct PublishReport {
    ...
    pub note: String,
    pub healthy: bool,
}
```

`healthy` is true whenever no stage failed, and a dry run fails no
stage, so a plan is healthy. A boolean cannot say "nothing was
attempted".

## The Rule

One rule, in the one place the publish report is built:

```rust
// src/publish/model.rs
/// The publish run's verdict.
///
/// The verdict is what the run did. A plan is not a delivery, and a
/// delivery that did not complete every stage is not a delivery
/// either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PublishVerdict {
    /// Every stage completed and the run delivered.
    Delivered,
    /// No write was attempted. The report is a plan.
    Planned,
    /// A stage did not complete.
    Failed,
}

impl PublishVerdict {
    pub fn label(self) -> &'static str {
        match self {
            PublishVerdict::Delivered => "delivered",
            PublishVerdict::Planned => "planned",
            PublishVerdict::Failed => "failed",
        }
    }
}

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

`PublishReport` gains the verdict, and `healthy` is derived from it:

| The report | `verdict` | `healthy` |
|---|---|---|
| every stage completed | `delivered` | `true` |
| a stage did not complete | `failed` | `false` |
| no write was attempted | `planned` | `false` |

### What each surface does with the verdict

| Surface | Before | After |
|---|---|---|
| `run_publish`'s journal row | `done` for a plan | `planned` for a plan |
| `run_publish`'s journal detail | `healthy={}` | `verdict={} healthy={}` |
| `cmd_publish`'s exit code | `healthy` decides | the verdict decides: `delivered` and `planned` exit 0, `failed` exits 1 |
| the fleet exec lane's journal row | `done` for a plan | `planned` for a plan |
| the fleet lane's per-project state | `done` for a plan | `planned` for a plan |
| the fleet lane's summary | `healthy: true` for a plan | the summary names the verdict |

### Why a plan is not a failure

The exit code follows the verdict, not `healthy`. A plan that ran to
completion is a successful plan and exits 0 exactly as it does today.
A failed publish still exits 1 with the report on stdout.

### Why the fleet lane's summary changes shape

The fleet lane's per-project summary carries `healthy`, which its
reader compares against the other projects'. With the verdict the
summary says `planned` for a plan, so a reader can tell "published"
from "planned" without reading the project's stage list. The `healthy`
key is kept, and it is now false for a plan.

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A dry run of any action | `planned`; the journal records `planned`; the exit code is 0. |
| A publish whose stages all completed | `delivered`; `healthy` is true; the journal records `done`; the exit code is 0. |
| A publish whose stage failed | `failed`; `healthy` is false; the journal records `failed`; the exit code is 1. |
| The fleet lane's manual path, a dry run | `planned` per project; the per-project state is `planned`; the fleet is not a failure. |
| The fleet lane's manual path, a failed publish | `failed` per project; the per-project state is `failed`. |
| The fleet lane's provider path | Unchanged: that branch reads the provider's own `health` field, which is its own evidence. |
| The fleet exec lane | `planned` for a plan; `done` for a delivery; `failed` for a failure. |
| The release lane's publish stage | Unchanged: it reads the report's note. |
| The MCP publish method | Unchanged: it returns the report, and the report now carries the verdict. |

## Risks

| Risk | Mitigation |
|---|---|
| A dry run starts exiting 1. | The exit code follows the verdict, not `healthy`; a contract test asserts a dry run exits 0. |
| A reader of `healthy` on a dry run breaks. | `healthy` already meant "the stages delivered"; a plan never delivered. The verdict is added alongside it, and the journal detail names it. |
| A reader of the journal state `done` for a dry run breaks. | The journal detail names the verdict and the dry run, so the row stays unambiguous; the state becomes `planned`. |
| The fleet lane starts counting a plan as a failure. | The fleet lane follows the verdict, and a contract test asserts a dry-run fleet publish is not a failure. |
| The fleet lane's summary shape breaks a reader. | The `healthy` key is kept; only its value changes for a plan, and the verdict is added beside it. |
| The provider branch changes. | It is explicitly out of scope: that branch's evidence is the provider's own `health` field. |
