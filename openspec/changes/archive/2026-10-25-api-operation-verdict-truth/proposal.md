# Change: api-operation-verdict-truth

## Why

The shared HTTP operation boundary finalised every successful request as
a completed operation.

```rust
// src/api/server.rs, before
match f(op_id, &registry) {
    Ok((value, op_id, project_id)) => {
        let detail = format!("{kind} completed");
        let _ = registry.finalize_operation(op_id, "done", &detail);
        Ok((value, op_id, project_id))
    }
```

The boundary does not know what the request did. It knows the closure
returned `Ok`, so it writes `done` and `"{kind} completed"`.

Two project routes run a **plan** when the request asks for one:

```rust
// src/api/handlers_portfolio.rs, before
// POST /v1/projects/{id}/deployments
let report = if dry_run {
    let plan = crate::deploy::engine::prepare_deploy(
        &project_dir, &manifest, &config, &deploy_request,
    )?;
    serde_json::to_value(&plan)...
} else { ... };
```

```rust
// src/api/handlers_changes.rs, before
// POST /v1/projects/{id}/upgrade
if dry_run {
    let registry = Registry::open(db_path)?;
    let plan = plan_upgrade(&registry, id, feature)?;
    ...
}
```

A deploy plan deploys nothing. An upgrade plan changes nothing — the
engine says so in its own words: *"Plan a single-project upgrade without
changing anything."* Both routes wrote a row that said the operation
completed:

```
sqlite> SELECT kind, state, detail FROM operations;
api.deploy|done|api.deploy completed
api.upgrade|done|api.upgrade completed
```

The repository already has the rule that says what those rows should
say. The deploy model's own verdict names the state:

```rust
// src/deploy/model.rs
pub fn journal_state(self) -> &'static str {
    match self {
        DeployVerdict::Delivered => "done",
        DeployVerdict::Planned => "planned",
        DeployVerdict::Failed => "failed",
        DeployVerdict::Blocked => "blocked",
    }
}
```

and the command's plan lane already reads it:

```rust
// src/cli/release.rs
let verdict = forge::deploy::verdict_for_plan(plan.ready);
registry.record_operation("deploy", &project_id, verdict.journal_state(), &detail);
```

So the CLI records `planned` for the same plan the API records as
`done`. The promoted capability already says which one is right:

> The registry journal row of a deploy plan SHALL record that run, and a
> plan SHALL NOT record a completed deployment, because a plan deploys
> nothing.

The API boundary never read that rule. It had no rule of its own.

This is the programme's rule class: Forge states something its own
gathered evidence does not support — the operation row claims a
completed operation for work the response body itself says did not
happen.

## What Changes

- **One rule for the operation row.** A new rule names what an operation
  earned: a completed operation did the work, a planned operation only
  planned, a blocked operation was refused before it changed anything and
  a failed operation did not finish. The boundary reads it and no route
  writes a state of its own.
- **The closure states what it earned.** The route hands the boundary the
  outcome of the request it ran, so the row's state and its detail follow
  the run instead of the closure's return type.
- **The plan routes read their own domain rule.** The deploy route reads
  the deploy verdict, the upgrade route states its plan, and the applied
  requests keep the completed operation they had.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The API operation boundary | `forge` | Yes | `src/api/server.rs`: the row's state and detail come from the one rule. |
| The operation rule | `forge` | Added | `src/api/operation.rs`: the one place that names an operation row's state. |
| The project deploy route | `forge` | Yes | `src/api/handlers_portfolio.rs`: the plan and the apply state their outcome. |
| The project upgrade route | `forge` | Yes | `src/api/handlers_changes.rs`: the plan states its outcome. |
| The admin deploy route | `forge` | Yes | `src/api/admin/deploy.rs`: the apply states the report's verdict. |
| The other eleven API routes | `forge` | Yes (one line each) | `src/api/handlers_changes.rs`, `src/api/project_management.rs`, `src/api/workbench.rs`, `src/api/admin/release.rs`: they state the completed operation they earn. |
| The deploy verdict rule | `forge` | Read only | `src/deploy/model.rs` (`DeployVerdict`, `verdict_for_report`, `verdict_for_plan`) is unchanged. |
| The CLI lanes | `forge` | Read only | Unchanged: the command's plan lane already records `planned`. |
| The registry journal | `forge` | Read only | Unchanged: the same table, the same append-only rows. |
| The idempotent replay | `forge` | Yes (rename only) | `src/api/server.rs`: the replay helper is renamed, because its old name would state something false. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §33 owns the release and publish workflow; the `core-http-api` capability owns the HTTP operation contract. |
| Contracts | No field is removed or renamed. The wire envelopes, the status codes and the operation payload are unchanged; only the journal row's state and detail change. |
| Callers | The fifteen API routes that run through the shared boundary. |
| Persistence | The `operations` table keeps its shape. A planned operation is a row like any other. |
| Integrations | None. No adapter, provider or repository is touched. |
| Failures | No new error code. A failed closure still records `failed` with the error text. |
| Tests | A new contract test `tests/api_operation_verdict_truth_contract.rs`; the existing API suites stay green. |
| Dependencies | None added. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `api-operation-verdict-truth` | forge | Rust | added | the rule that names an API operation row's state. |
| `deploy-plan-verdict-truth` | forge | Rust | modified | the deploy rule's "every writer" scenario now names the API route that reads it. |

## Non-goals

- **The CLI lanes are unchanged.** The command's plan lane already records
  `planned`.
- **The deploy verdict rule is unchanged.** It stays the one rule that
  judges a deploy; this change only makes the API read it.
- **The registry journal is unchanged.** No new column, no new table.
- **The failure path of the boundary is unchanged.** A closure that
  returns `Err` still records `failed` with the error text.
- **The HTTP envelopes are unchanged.** No status code and no response
  field changes.
- **The web lifecycle helper is unchanged.** `src/api/lifecycle_exec/mod.rs`
  writes its own rows with its own rule; it is a different lane and it is
  recorded as the next candidate.
