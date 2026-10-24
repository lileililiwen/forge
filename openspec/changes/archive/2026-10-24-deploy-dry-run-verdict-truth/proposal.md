# Change: deploy-dry-run-verdict-truth

## Why

`forge deploy apply --dry-run` reports that the deploy happened.

Reproduced on the current tree, with a fixture adapter that rehearses
and answers `apply_status: delivered`:

```
$ forge deploy apply /tmp/dep-probe/proj --confirm --dry-run
...
stages:
  - apply target=`home` delivered: fixture adapter applied the artifact
  - observe target=`home` running: docker service app is running
summary: deploy applied to target `home` and observed as `running`
```

and the append-only row the same run wrote:

```
('deploy', 'dep-probe', 'done', 'deploy `dep-probe-home-b6630d40797e` target=`home` adapter=`local` stages=apply=delivered,observe=running healthy=true dry_run=true')
```

Nothing was deployed. The rehearsal wrote no deploy state at all —
`.forge/deploy` does not exist after the run — yet the summary says
"deploy applied to target `home`", the row says the deploy was `done`,
and it says `healthy=true`. The row even carries `dry_run=true`: one
line that knows it was a rehearsal and claims a completed, healthy
deployment in the same breath.

**The report's note is Forge's own claim, and it is false.**

```rust
// src/deploy/engine/stages.rs, before
let note = if healthy {
    format!(
        "deploy applied to target `{}` and observed as `{}`",
        plan.target.name, observation.status
    )
} else {
    ...
};
```

**The health flag is Forge's own claim, and it is unsupported.** The
stages are healthy because the adapter *rehearsed* successfully; a
rehearsal is not evidence that anything is running.

```rust
// src/deploy/engine/stages.rs, before
let healthy = stages.iter().all(|s| {
    s.status == STATUS_DELIVERED
        || s.status == STATUS_SKIPPED
        || s.status == STATUS_DISABLED
        || s.status == STATUS_RUNNING
});
```

**The journal state is Forge's own claim, and it is false.** A boolean
has two values, so a rehearsal had to be `done` or `partial`:

```rust
// src/cli/publish.rs, before
let state_label = if report.healthy() { "done" } else { "partial" };
let _ = registry.record_operation("deploy", &project_id, state_label, &detail);
```

**The exit code would follow it too.** `cmd_deploy_apply` returns
`DeployHealthFailed` when the report is not healthy, so a report that
stopped claiming health would turn every rehearsal into a failed
command unless the exit code follows the verdict instead.

This is the rule the repair programme is built on: Forge states
something its own gathered evidence does not support. A rehearsal
gathers no runtime state, and Forge must not say it applied and
observed.

## What Changes

- **One rule judges a deploy run**, in the one place the report is
  built: a typed verdict — `delivered`, `planned` or `failed` — read
  from the dry-run flag and the stage outcomes.
- **A rehearsal is a plan.** The report's note says what was
  rehearsed and that nothing was deployed; the report is not healthy;
  the journal records a planned deploy.
- **The exit code follows the verdict.** A plan is a successful plan
  and exits 0, exactly as the rehearsal does today. A failed deploy
  still exits 1 with the report on stdout.
- **The API's deploy view carries the verdict**, so the browser reads
  the same rule as the CLI.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The verdict rule | `forge` | Yes | `src/deploy/model.rs`: `DeployVerdict`, `verdict_for_report`, `DeployReport.verdict`. |
| The deploy apply | `forge` | Yes | `src/deploy/engine/stages.rs`: the note, the health, the journal detail. |
| The deploy CLI | `forge` | Yes | `src/cli/publish.rs`: the journal state and the exit code follow the verdict. |
| The API's deploy view | `forge` | Yes | `src/api/admin/deploy_views.rs`: the view carries the verdict. |
| The deploy unit tests | `forge` | Yes | `src/deploy/engine/stages_tests.rs`: the verdict's unit coverage. |
| The deploy contract test | `forge` | Added | `tests/deploy_dry_run_verdict_truth_contract.rs`. |
| The `deploy-dry-run-verdict-truth` capability | `forge` | Added | this package |
| The `adapter-deployment` capability | `forge` | Modified | the health requirement names the verdict. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §26 owns the deploy lane; the `adapter-deployment` capability owns the deploy report and its health. |
| Contracts | The report gains one field, `verdict`. No field is removed or renamed, and `DEPLOY_CONTRACT_VERSION` is unchanged. |
| Callers | `forge deploy apply` (human and JSON), its journal row and its exit code; the API's deploy report view. |
| Persistence | The registry journal state for a rehearsal becomes `planned`. The deploy state file is unchanged: a rehearsal still writes nothing. |
| Integrations | None. The adapter is still spawned with the same argv and the same `dry_run` flag; the adapter's own stage statuses and notes are still recorded exactly as its envelope names them. |
| Failures | No new error code. A rehearsal still exits 0; a failed deploy still exits 1 with `DeployHealthFailed`. |
| Tests | A new contract test `tests/deploy_dry_run_verdict_truth_contract.rs`; the deploy unit tests gain the verdict's coverage; the existing deploy contract and cross-surface tests stay green. |
| Dependencies | None added. |
| Concerns | Quality: the journal and the summary stop claiming a deployment that never happened. Compatibility: a reader of the report's `healthy` now reads `false` for a rehearsal; a reader of the journal state `done` for a rehearsal now reads `planned`; the report gains a `verdict` field. Security: no new surface; the evidence redaction rule and the path scrubbing are unchanged. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `deploy-dry-run-verdict-truth` | forge | Rust | added | this package |
| `adapter-deployment` | forge | Rust | modified | the health requirement names the verdict. |

## Non-goals

- **The adapter's own claims are untouched.** The stage statuses and
  the stage notes are the adapter's envelope, recorded exactly as the
  canonical contract requires. Forge's own verdict is what changes.
- **The rehearsal still contacts nothing.** A rehearsal still spawns
  the adapter with `dry_run: true`, still writes no deploy state and
  still refuses nothing.
- **The observe lane is not in this package.** `forge deploy observe`
  never runs a rehearsal.
- **The delivery rule is not in this package.** A rehearsal's refusal
  of the ship gate is unchanged.
- **The deploy state file is not in this package.** A rehearsal writes
  none, exactly as today.
- **The deploy contract version is unchanged.**
