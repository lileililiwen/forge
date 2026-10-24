# Design: deploy-plan-verdict-truth

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The run rule | `forge` | Yes. `src/deploy/model.rs`. |
| The deploy plan CLI | `forge` | Yes. `src/cli/release.rs`. |
| The deploy apply CLI | `forge` | Read only. |
| The deploy contract test | `forge` | Added. `tests/deploy_plan_verdict_truth_contract.rs`. |
| The `deploy-plan-verdict-truth` capability | `forge` | Added. |
| The `adapter-deployment` capability | `forge` | Modified. |
| The deploy plan API route | `forge` | Read only. |
| The deploy state file | `forge` | Read only. |
| The deploy contract version | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/cli/release.rs, before
let state_label = if plan.healthy() { "done" } else { "blocked" };
let _ = registry.record_operation("deploy", &project_id, state_label, &detail);
```

### The row it wrote

```
('deploy', 'dep-probe', 'done', 'deploy plan `dep-probe-home-b6630d40797e` target=`home` adapter=`local` ready=true')
```

Nothing was deployed. The adapter never ran, the artifact was never
applied, and `.forge/deploy` did not exist afterwards.

### The second copy of the rule

```rust
// src/cli/publish.rs — the deploy apply
let state_label = report.verdict.journal_state();
```

The apply lane reads the model; the plan lane keeps a private copy of
"what did this run do", and that copy has no plan arm at all.

### The canonical rule the plan lane already breaks

The `adapter-deployment` capability says, in the requirement *Observed
health and recovery*:

> The deploy report SHALL carry a verdict that names what the run did,
> so a run that deployed nothing is reported and journalled as a plan
> and never as a completed deployment.

A deploy plan deploys nothing. It was journalled as a completed
deployment. The same requirement also reaches the browser: the admin
delivery timeline renders every `deploy` operation row with its state,
so the operator read a completed deployment that never happened.

## The Rule

One type names what a deploy run did, in the one place the deploy model
lives. Change 36 gave it the run's three arms; this change adds the
plan's own arm:

```rust
// src/deploy/model.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeployVerdict {
    /// The deploy ran and every stage completed.
    Delivered,
    /// No deploy was attempted. The report is a rehearsal.
    Planned,
    /// A stage did not complete.
    Failed,
    /// A plan the shared delivery rule blocks. A plan never reaches an
    /// adapter, so this arm belongs to the plan lane alone: the report
    /// of an apply never carries it.
    Blocked,
}

impl DeployVerdict {
    pub fn label(self) -> &'static str {
        match self {
            DeployVerdict::Delivered => "delivered",
            DeployVerdict::Planned => "planned",
            DeployVerdict::Failed => "failed",
            DeployVerdict::Blocked => "blocked",
        }
    }

    pub fn journal_state(self) -> &'static str {
        match self {
            DeployVerdict::Delivered => "done",
            DeployVerdict::Planned => "planned",
            DeployVerdict::Failed => "failed",
            DeployVerdict::Blocked => "blocked",
        }
    }
}

/// The one rule that judges a deploy plan.
///
/// A plan deploys nothing: it names the target, the artifact and the
/// revision, and asks the shared delivery rule whether the revision may
/// go out. A plan the rule allows is a planned run; a plan the rule
/// blocks is a blocked run. Neither is a completed deployment.
pub fn verdict_for_plan(ready: bool) -> DeployVerdict {
    if ready {
        DeployVerdict::Planned
    } else {
        DeployVerdict::Blocked
    }
}
```

### The writer reads it

```rust
// src/cli/release.rs, the deploy plan
let verdict = forge::deploy::verdict_for_plan(plan.ready);
let detail = format!(
    "deploy plan `{}` target=`{}` adapter=`{}` ready={} verdict={}",
    plan.identity.id, plan.target.name, plan.target.kind, plan.ready, verdict.label()
);
let _ = registry.record_operation("deploy", &project_id, verdict.journal_state(), &detail);
```

The apply lane is untouched: it still reads `report.verdict.journal_state()`.

### The options, decided

| Option for the deploy plan's journal row | Verdict |
|---|---|
| Keep `done` and document that "done" means "the operation finished" | Rejected. `done` is the state a delivered deploy writes after every stage completed. A plan would be indistinguishable from a delivery, and the canonical `adapter-deployment` requirement already requires a run that deployed nothing to be journalled as a plan. |
| `planned` for a ready plan, `blocked` for a plan the delivery rule blocks | **Chosen.** The lane already has `planned` — change 36 gave the rehearsal that state — and the state then names what the run did. |
| A new state `prepared` | Rejected. New vocabulary for a plan the lane already names `planned`; it adds a word, not a truth. |
| Stop journalling the plan | Rejected. The journal is the product's only record that a plan was computed and for which revision; dropping the row loses evidence, and the admin delivery timeline would lose the plan from the operator's history. |
| A separate `DeployPlanRun` type next to `DeployVerdict` | Rejected. Two types for one lane's journal vocabulary is two vocabularies. One type with the plan's own arm keeps the deploy journal's states in one place, and mirrors the release lane, where change 37 gave `ReleaseRun` both the plan's arms and the apply's arms. |

### Why the plan's arm is on the same type

The deploy lane writes one journal kind, `deploy`, from two writers: the
plan and the apply. A reader of that journal sees both. The vocabulary
that describes them must therefore be one vocabulary, and the change 37
precedent is the same shape: `ReleaseRun` carries `Planned` and
`Blocked` for the release prepare and `Delivered` and `Partial` for the
release apply.

### Why the blocked case keeps its state

A plan the shared delivery rule blocks is not a plan the deploy may
take. `blocked` says the deploy cannot proceed and names the rule's own
reason, which the plan's note already carries. The change only removes
the false claim: a plan that *can* proceed is not a deploy that *did*.

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A ready deploy plan | the run is `planned`; the row records `planned` with `verdict=planned` in its detail. |
| A deploy plan the shared delivery rule blocks | the run is `blocked`; the row records `blocked`, as before. |
| A deploy apply that deployed everything | the run is `delivered`; the row records `done`, as before. |
| A deploy apply that deployed nothing | the run is `planned`; the row records `planned`, as before. |
| A deploy apply whose stage did not complete | the run is `failed`; the row records `failed`, as before. |
| The deploy plan API route | unchanged: it is a read-only view, it journals nothing and its view carries `ready` and the delivery rule's reason. |
| The deploy state file | unchanged: a plan writes none. |
| The exit codes | unchanged: a plan exits 0, ready or blocked. |
| The report's `verdict` field | unchanged: `verdict_for_report` still returns only `Delivered`, `Planned` and `Failed`, so a report never carries the plan's arm. |
| The report's exhaustive matches | The report's note and the command's exit code match on the verdict. The plan's arm is spelled out in those matches with a comment that says it is unreachable, because `verdict_for_report` cannot produce it. The arm behaves as the failure arm, so the compiler keeps the match exhaustive without a wildcard. |

## Risks

| Risk | Mitigation |
|---|---|
| A reader of the deploy journal's `done` state for a plan breaks. | The value was false: a plan deployed nothing. The new state says what the run did, and the detail keeps naming the command and the readiness. |
| A reader counts `state='done'` deploy rows to count deployments. | That count was wrong before this change: it counted plans as deployments. It is right afterwards. |
| The report could now carry the plan's arm. | `verdict_for_report` is unchanged and never returns `Blocked`; the arm's doc comment says it belongs to the plan lane alone. |
| The detail's format changes. | The detail gains one `verdict=` field; every other field keeps its name and its order. |
| The apply lane's rows change. | Their states do not change at all; only the detail gains the `verdict=` field, and change 36 already gave it that field. |
| The deploy contract version is unchanged. | Nothing is removed or renamed in the report, the plan or the view. |
