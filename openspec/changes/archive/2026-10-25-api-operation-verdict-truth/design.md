# Design: api-operation-verdict-truth

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The operation rule | `forge` | Added. `src/api/operation.rs`. |
| The API operation boundary | `forge` | Yes. `src/api/server.rs`. |
| The project deploy route | `forge` | Yes. `src/api/handlers_portfolio.rs`. |
| The project upgrade route | `forge` | Yes. `src/api/handlers_changes.rs`. |
| The admin deploy route | `forge` | Yes. `src/api/admin/deploy.rs`. |
| The other eleven API routes | `forge` | Yes, one line each. |
| The API contract test | `forge` | Added. `tests/api_operation_verdict_truth_contract.rs`. |
| The `api-operation-verdict-truth` capability | `forge` | Added. |
| The `deploy-plan-verdict-truth` capability | `forge` | Modified. |
| The deploy verdict rule, the CLI lanes, the registry journal | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/api/server.rs, before
match f(op_id, &registry) {
    Ok((value, op_id, project_id)) => {
        let detail = format!("{kind} completed");
        let _ = registry.finalize_operation(op_id, "done", &detail);
        Ok((value, op_id, project_id))
    }
```

The boundary's claim is that the closure's `Ok` means the work was
done. The closure returns `Ok` for a plan too.

### What it produced

```
sqlite> SELECT kind, state, detail FROM operations;
api.deploy|done|api.deploy completed
api.upgrade|done|api.upgrade completed
```

### The evidence, and where it is not

| The fact | Where it lives |
|---|---|
| The deploy request planned | The response body: the route returns the `DeployPlan` (`ready`, `ship_gate`, `note`), not a report. |
| The deploy applied nothing | `prepare_deploy` never contacts an adapter and never writes the deploy state. |
| The upgrade request planned | `plan_upgrade`'s own doc: *"Plan a single-project upgrade without changing anything."* |
| The CLI records the same plan as `planned` | `src/cli/release.rs`: the plan lane journals `verdict_for_plan(plan.ready).journal_state()`. |
| The rule that says so | `src/deploy/model.rs`: `DeployVerdict::journal_state()` maps `Planned` to `planned`. |

The operation row claimed a completed operation for two requests whose
own bodies say nothing was applied.

## The Rule

**An operation row's state names what the operation did: a completed
operation did the work, a planned operation only planned, a blocked
operation was refused before it changed anything, a failed operation did
not finish.**

### The rule lives in one place, next to the boundary that writes the row

```rust
// src/api/operation.rs, after
/// What an operation row earned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationOutcome {
    /// The operation did the work it was asked to do.
    Completed,
    /// The operation only planned: nothing was applied.
    Planned,
    /// The shared rule refused the operation before it changed anything.
    Blocked,
    /// The operation did not finish.
    Failed,
}

impl OperationOutcome {
    /// The state the journal row records.
    pub fn journal_state(self) -> &'static str { ... }

    /// The detail the journal row carries. The detail names the run, so
    /// a reader never has to guess what the state means.
    pub fn detail(self, kind: &str) -> String { ... }

    /// The outcome of a deploy run, read from the deploy model's own
    /// verdict: the deploy rule judges the run, this rule only names the
    /// row's state.
    pub fn from_deploy_verdict(verdict: DeployVerdict) -> Self { ... }
}
```

The rule is one enum with one `journal_state` and one `detail`. The
boundary reads it; no route writes a state string of its own.

### The closure states what it earned

```rust
// src/api/server.rs, after
pub(in crate::api) fn run_with_operation<T, F>(
    db_path: &Path,
    kind: &str,
    project_id: &str,
    request: &ApiRequest,
    f: F,
) -> Result<(T, i64, String), ApiResponse>
where
    F: FnOnce(i64, &Registry) -> Result<(T, i64, String, OperationOutcome), ForgeError>,
```

```rust
// src/api/server.rs, after
match f(op_id, &registry) {
    Ok((value, op_id, project_id, outcome)) => {
        // The one rule: the row's state names what the operation did and
        // the detail names the run.
        let detail = outcome.detail(kind);
        let _ = registry.finalize_operation(op_id, outcome.journal_state(), &detail);
        Ok((value, op_id, project_id))
    }
```

### The plan routes read their own domain rule

```rust
// src/api/handlers_portfolio.rs, after
let (value, outcome) = if dry_run {
    let plan = crate::deploy::engine::prepare_deploy(
        &project_dir, &manifest, &config, &deploy_request,
    )?;
    let outcome = OperationOutcome::from_deploy_verdict(
        crate::deploy::verdict_for_plan(plan.ready),
    );
    let value = serde_json::to_value(&plan).map_err(|err| ForgeError::Registry {
        reason: err.to_string(),
    })?;
    (value, outcome)
} else {
    let report = crate::deploy::engine::apply_deploy(
        &project_dir, &manifest, &config, &deploy_request, &adapter,
    )?;
    let outcome = OperationOutcome::from_deploy_verdict(report.verdict);
    let value = serde_json::to_value(&report).map_err(|err| ForgeError::Registry {
        reason: err.to_string(),
    })?;
    (value, outcome)
};
```

The upgrade route has no domain verdict of its own, so it states the
one thing it knows:

```rust
// src/api/handlers_changes.rs, after
if dry_run {
    let registry = Registry::open(db_path)?;
    let plan = plan_upgrade(&registry, id, feature)?;
    let value = serde_json::to_value(&plan).map_err(|err| ForgeError::Registry {
        reason: err.to_string(),
    })?;
    return Ok((value, op_id, id.to_string(), OperationOutcome::Planned));
}
```

### The applied requests keep the completed operation they had

The eleven routes that really do the work state it:

```rust
Ok((value, op_id, id.to_string(), OperationOutcome::Completed))
```

### The replay helper is renamed

`pending_or_done_response` said what it no longer does: a replayed
operation can now be `planned`, `blocked` or `failed`. The helper is
renamed `replay_response` and its comment states the rule it reads.

## The options, decided

| Option for how the closure states the outcome | Verdict |
|---|---|
| The closure returns the outcome in its tuple | **Chosen.** Every route states what it earned, and the boundary cannot guess. The compiler names every writer that has to answer. |
| A second helper for the plan routes (`run_with_planned_operation`) | Rejected. Two entry points into one rule: a route author picks the wrong one and nothing says so. The defect was a route that did not know it had to state anything. |
| A `plan: bool` parameter on the boundary | Rejected. The flag carries the same information as the outcome and lets a caller pass `false` by accident, with no compiler help. |
| The boundary derives the outcome from the returned value | Rejected. The boundary would have to know every route's value type; a new route type would silently fall back to the wrong arm. |
| A hand-written state string at each call site | Rejected. That is the rule class this programme exists to remove: a state a reader has to trust instead of a rule they can read. |

| Option for the rule's home | Verdict |
|---|---|
| A new module `src/api/operation.rs` | **Chosen.** One rule, one file, next to the boundary that writes the row. The module is small and the rule has one reader family. |
| Inside `src/api/server.rs` | Rejected. `server.rs` is the transport; the rule is the vocabulary every route answers in, and it deserves its own file. |
| Inside the deploy model | Rejected. The rule is not the deploy model's: it names an API operation row, and the upgrade route has no deploy verdict at all. |

| Option for the rule's arms | Verdict |
|---|---|
| `Completed` and `Planned` only | Rejected. The deploy route's blocked plan and failed report are two more things the row can honestly say, and the deploy model already names both. |
| `Completed`, `Planned`, `Blocked` and `Failed` | **Chosen.** The four states are the ones the journal already uses, and each one has a fact behind it. |
| Keep a single "done" and add a free-form note | Rejected. The state is what a reader filters on; a note cannot replace it. |

| Option for the deploy route's failed report | Verdict |
|---|---|
| Record the report's verdict, so a failed deploy records `failed` | **Chosen.** The promoted capability already says the journal state follows the report's verdict, for the CLI and the API. |
| Record `done` for every apply that returned `Ok` | Rejected. `apply_deploy` returns `Ok` with a failed verdict; the row would claim a delivery the report denies. |

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A request that applied the work | A completed operation: `done`, `{kind} completed`. |
| A request that only planned | A planned operation: `planned`, `{kind} planned`. |
| A request the shared rule refused | A blocked operation: `blocked`, `{kind} blocked`. |
| A request whose run did not finish | A failed operation: `failed`, `{kind} failed`. |
| A closure that returns `Err` | Unchanged: `failed` with the error text as the detail. |
| An idempotent replay of a finished request | Unchanged: `200` with the recorded row, whose state is the one the rule recorded. |
| An idempotent replay of a pending request | Unchanged: `202`. |
| The response envelopes | Unchanged: no status code and no field changes. |
| The CLI lanes | Unchanged. |

## Risks

| Risk | Mitigation |
|---|---|
| A new route forgets to state an outcome. | It does not compile: the closure's return type changed, so every writer is named by the compiler. |
| The state vocabulary drifts from the rest of the repository. | The four states are the ones the journal already uses, and the deploy route's arm comes from the deploy model's own `journal_state`. The contract test pins the vocabulary. |
| The replay helper's old name survives. | It is renamed in the same change; the old name would state something false. |
| The failed apply changes a row that a reader depends on. | The row said `done` for a report that said `verdict: failed`; the new state is the report's own verdict, which the promoted capability already requires. |
| The diff touches fifteen routes. | Thirteen of them change one line — the outcome they earn — and the behaviour of the eleven non-deploy routes is unchanged. |
