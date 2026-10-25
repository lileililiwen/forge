# Design: planner-plan-verdict-truth

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The intent resolve lane | `forge` | Yes. `src/cli/intent.rs`. |
| The planner apply lane | `forge` | Yes. `src/planner/apply.rs`. |
| The planner contract test | `forge` | Added. `tests/planner_plan_verdict_truth_contract.rs`. |
| The `resolve-plan-verdict-truth` capability | `forge` | Modified. |
| The `validated-intent-planner` capability | `forge` | Modified. |
| The shared resolution rule | `forge` | Read only. `src/core/resolve.rs`. |
| The plan receipt and the plan steps | `forge` | Read only. |
| The apply gate | `forge` | Read only. |
| The component and UI pattern lanes | `forge` | Read only. |
| The agent lane | `forge` | Read only. |

## The Gap

### The claims

```rust
// src/cli/intent.rs, before — the resolve writer
let _ = registry.record_operation(
    "planner",
    &plan.plan_id,
    "done",
    &format!("resolve: {}", plan.note),
);
```

```rust
// src/planner/apply.rs, before — the component step
match resolve_components(&request) {
    Ok(_) => applied.push(AppliedStep {
        kind: step.kind,
        target: step.target.clone(),
        status: "verified".to_string(),
        note: format!(
            "component {}@{} resolved for {}; promotion is the operator's \
             call and the planner did not auto-qualify it",
            step.target, step.version, plan.profile
        ),
    }),
```

### What they produced

```
('planner', 'rust-web-1ae2bc11-a90ac786', 'done', 'resolve: deterministic assembly plan for create_project: 3 step(s), 0 unresolved, profile rust-web@0.1.0')
```

and, in the apply report, a step whose status says `verified` beside a
note that says the planner resolved the component and qualified
nothing.

### The evidence, and where it is not

| The fact | Where it lives |
|---|---|
| The plan the operator reviews | The plan and its receipt under `.forge/planner/<plan-id>/plan.json`. |
| The steps the apply performed | The `applied` array of the apply report, one entry per step. |
| What a resolve did | Nothing was applied: `forge intent apply` is a separate command with its own confirm gate. |
| What the component step did | A catalog resolve. The step's own note says the promotion is the operator's call. |

### The rule that already exists

Change 42 put the resolution rule in the shared Core and promoted it:

```rust
// src/core/resolve.rs
pub fn for_steps(step_count: usize) -> Self {
    if step_count == 0 {
        ResolveRun::Rejected
    } else {
        ResolveRun::Planned
    }
}
```

> Every journal row for a resolution SHALL follow that one rule rather
> than a private copy of it, and the row's detail SHALL name the run.

The intent planner is a resolution writer that never joined it.

## The Rule

**The planner names the action it performed: a resolve is a planned run,
and a resolved component is a resolved component.**

### The resolve lane joins the one rule

```rust
// src/cli/intent.rs, after
// One rule names the run: a resolve produced a plan and applied
// nothing, so it is a planned run and never a completed resolution.
let run = ResolveRun::for_steps(plan.steps.len());
if let Ok(registry) = open_registry(db_path) {
    let _ = registry.record_operation(
        "planner",
        &plan.plan_id,
        run.journal_state(),
        &format!("resolve: verdict={} {}", run.label(), plan.note),
    );
}
```

The row after the change:

```
('planner', 'rust-web-1ae2bc11-a90ac786', 'planned', 'resolve: verdict=planned deterministic assembly plan for create_project: 3 step(s), 0 unresolved, profile rust-web@0.1.0')
```

The command's output names the run too, because the promoted rule says
the output names it: the JSON envelope gains the note the row carries,
and the human render ends with the same line.

```rust
// src/cli/intent.rs, after
let json = serde_json::json!({
    "contract": PLANNER_CONTRACT_VERSION,
    "note": note.clone(),
    "plan": plan,
    "receipt_path": outcome.receipt_path,
});
let human = format!("{}\n  {}", render_plan_human(&outcome.plan), note);
```

The plan and the receipt are unchanged; only the command's output and
the journal row gain the run's name.

### The apply lane names the action it performed

```rust
// src/planner/apply.rs, after
match resolve_components(&request) {
    Ok(_) => applied.push(AppliedStep {
        kind: step.kind,
        target: step.target.clone(),
        // The step resolved the component in the catalog. It
        // installed nothing and qualified nothing, so it does not
        // report a verified component.
        status: "resolved".to_string(),
        note: format!(
            "component {}@{} resolved for {}; promotion is the operator's \
             call and the planner did not auto-qualify it",
            step.target, step.version, plan.profile
        ),
    }),
```

The step's status now agrees with its own note.

## The options, decided

| Option for the resolve row's state | Verdict |
|---|---|
| Keep `done` | Rejected. A resolve applies nothing; `done` is the state for a completed action, and the promoted rule already says a resolution is never recorded as one. |
| Keep `done` and add "plan" to the detail | Rejected. The state is what a reader counts and filters on; a plan behind a `done` state is still a completed resolution on the record. |
| Read the shared `ResolveRun` rule | **Chosen.** The rule already exists in the Core; this lane becomes a third reader of it, exactly as the promoted capability says every resolution writer must be. |
| Give the planner its own verdict type | Rejected. A second copy of one rule is where the lanes drift; the two copies already drifted once. |
| Journal no row at all | Rejected. The plan was produced and reviewed; the transition is worth a record, and the plan's own receipt is the artifact. |

| Option for the component step's status | Verdict |
|---|---|
| Keep `verified` | Rejected. The step resolved a component in the catalog. It installed nothing, qualified nothing, and its own note says the promotion is the operator's call. |
| Report `resolved` | **Chosen.** The status names the action the step performed, and it agrees with the note beside it. |
| Report `installed` | Rejected. Nothing was installed; the planner has no component installer. |
| Report `skipped` | Rejected. The step did not skip anything: the resolve ran and succeeded. |
| Report `applied` | Rejected. The planner applied no component; `applied` is the status of a feature install. |

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A resolve that produced steps | The row is `planned` and names the run. |
| A resolve that produced no step | The row is `rejected`, by the same shared rule. |
| A resolve's command output | Names the run, like the other two resolver lanes. |
| An apply whose component step resolved | The step's status is `resolved`, and the note stays. |
| An apply whose feature step installed | Unchanged: the status is `applied`. |
| An apply whose UI pattern step installed | Unchanged: the status is `installed`. |
| An apply whose gate steps are scheduled | Unchanged: the status is `scheduled`. |
| A stale plan | Unchanged: the apply refuses it. |
| The plan and its receipt | Unchanged. |
| The other lanes' rows | Unchanged. |

## Risks

| Risk | Mitigation |
|---|---|
| A reader that counted `done` planner rows now counts `planned` rows. | That count was wrong: it counted plans as applied resolutions. The release and deploy lanes and now the component and UI pattern lanes all use `planned` for a plan. |
| The applied step's status changed from `verified` to `resolved`. | Nothing in the repository asserts the old value, and the new value is the one the step's own note describes. The plan and the apply gate are unchanged. |
| A second copy of the resolution rule could appear in the planner lane. | The planner reads `ResolveRun::for_steps` from the shared Core; the decision stays in one place. |
| The component step is not reachable from the deterministic resolver today. | No supported profile declares a component id as a capability, so the resolver cannot emit a component step on its own. The false claim is still in shipped code, and the contract test drives the executor through its public contract with the step built on a resolved plan. |
