# Change: planner-plan-verdict-truth

## Why

The intent planner makes two claims its own evidence does not support.

**A resolved plan is journalled as a completed resolution.**

```
$ forge --registry /tmp/plan-probe/registry.db intent resolve --action create_project --profile rust-web
  "plan": { "steps": [ ...3 steps... ], "note": "deterministic assembly plan for create_project: 3 step(s), 0 unresolved, profile rust-web@0.1.0" }

$ sqlite3 /tmp/plan-probe/registry.db "SELECT kind, state, detail FROM operations"
('planner', 'rust-web-1ae2bc11-a90ac786', 'done', 'resolve: deterministic assembly plan for create_project: 3 step(s), 0 unresolved, profile rust-web@0.1.0')
```

`forge intent resolve` resolves an intent into a plan and writes the
plan's receipt. It applies nothing: `forge intent apply` is a separate
command with its own confirm gate. The journal row says `done` — the
state the registry uses for a completed action — for a plan that was
never applied.

The state was chosen by a private copy of the rule, in a third lane:

```rust
// src/cli/intent.rs, before
let _ = registry.record_operation(
    "planner",
    &plan.plan_id,
    "done",
    &format!("resolve: {}", plan.note),
);
```

Change 42 already gave this programme the shared rule, in the shared
Core, and named the two lanes that read it:

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

The promoted capability `resolve-plan-verdict-truth` says: *"Every
journal row for a resolution SHALL follow that one rule rather than a
private copy of it"* and names the component resolver and the UI
pattern resolver. The intent planner is a third resolution writer and
it never joined them.

**A resolved component is reported as verified.**

```rust
// src/planner/apply.rs, before
PlanStepKind::InstallComponent => {
    let request = crate::component::ComponentRequest { ... };
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

The step resolves a component in the catalog. It installs nothing, it
qualifies nothing, and the step's own note says so — while the status
next to it says `verified`. The row contradicts itself, the way the
agent lane's rows did before change 44.

Both are the programme's rule class: Forge states something its own
gathered evidence does not support.

## What Changes

- **The planner joins the one resolution rule.** The intent resolver
  reads the shared `ResolveRun` rule from the Core: a plan is a
  `planned` run, never `done`, and the row names the run. The command's
  output names it too, like the other two resolver lanes.
- **A resolved component is reported as resolved.** The apply lane
  names the action it performed. `verified` is not one of them.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The intent resolve lane | `forge` | Yes | `src/cli/intent.rs`: the row reads the shared rule. |
| The planner apply lane | `forge` | Yes | `src/planner/apply.rs`: the step names the action it performed. |
| The shared resolution rule | `forge` | Read only | `src/core/resolve.rs` (`ResolveRun`) already owns the decision; this change makes a third writer read it. |
| The plan receipt | `forge` | Read only | The plan and its receipt are unchanged. |
| The plan steps | `forge` | Read only | A plan still names the action it would perform. |
| The `intent apply` gate | `forge` | Read only | The staleness re-check and the confirm gate are unchanged. |
| The planner contract test | `forge` | Modified | `tests/planner_contract.rs`: the row and the step status follow the new rule. |
| The planner contract test | `forge` | Added | `tests/planner_plan_verdict_truth_contract.rs`. |
| The `resolve-plan-verdict-truth` capability | `forge` | Modified | the rule names every resolution writer. |
| The `validated-intent-planner` capability | `forge` | Modified | the lane's requirements state what it recorded. |
| The component and UI pattern lanes | `forge` | Read only | Unchanged by this change (change 42 owns them). |
| The agent lane | `forge` | Read only | Unchanged by this change (change 44 owns it). |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §16 and §17 own the intent planner; the `validated-intent-planner` capability owns the plan and the apply, and `resolve-plan-verdict-truth` owns the shared rule. |
| Contracts | No field is removed or renamed. The planner journal row's `state` column carries the planned run, and one applied step's `status` carries `resolved` instead of `verified`. |
| Callers | `forge intent resolve` and `forge intent apply`. |
| Persistence | The planner's journal rows read `planned` instead of `done`; the plan receipt is unchanged. |
| Integrations | None. No adapter, provider or repository is touched. |
| Failures | No new error code and no new refusal. A resolve still answers with its plan and a receipt; an apply still refuses a stale plan. |
| Tests | A new contract test `tests/planner_plan_verdict_truth_contract.rs`; the existing planner tests stay green. |
| Dependencies | None added. |
| Concerns | Quality: the operations table stops claiming completed resolutions that were never applied. Compatibility: the plan, the receipt and the apply gate are unchanged. Security: no new surface. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `resolve-plan-verdict-truth` | forge | Rust | modified | the rule names every resolution writer, the intent planner included. |
| `validated-intent-planner` | forge | Rust | modified | the lane's requirements state what it recorded. |

No new capability: the rule already exists in `resolve-plan-verdict-truth`,
and a second copy of it would be the defect this programme keeps
repairing.

## Non-goals

- **The plan is unchanged.** A plan still names the action it would
  perform; a plan is a plan.
- **The apply gate is unchanged.** The staleness re-check, the confirm
  gate and the partial-run report are untouched.
- **The other step kinds are unchanged.** A feature is still `applied`,
  a UI pattern still `installed`, a scheduled gate still `scheduled`.
- **The component and UI pattern lanes are unchanged.**
- **The agent lane is unchanged.**
- **No journal row is rewritten.** Rows written before this change stay
  as they are.
