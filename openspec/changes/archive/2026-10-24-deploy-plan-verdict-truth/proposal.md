# Change: deploy-plan-verdict-truth

## Why

`forge deploy plan` records a **completed deployment** in the registry
journal for a plan that deployed nothing.

Reproduced on the current tree, with a project whose Gate evidence
satisfies the shared delivery rule:

```
$ forge deploy plan /tmp/dep-probe/proj
project: dep-probe
deploy_id: dep-probe-home-b6630d40797e
target: home (local)
ready: yes
summary: deploy plan ready for target `home` (kind `local`) at revision `0faabe2b0603`
```

and the append-only row the same run wrote:

```
('deploy', 'dep-probe', 'done', 'deploy plan `dep-probe-home-b6630d40797e` target=`home` adapter=`local` ready=true')
```

`state='done'`. Nothing was deployed: no adapter ran, no artifact was
applied, and `.forge/deploy` does not exist. The plan's own note says
`deploy plan ready` — but the state next to it says the deployment
happened.

**The lane already knows the difference.** `forge deploy apply
--dry-run` records `planned` (change 36), and the canonical
`adapter-deployment` requirement already says that a run which deployed
nothing is *journalled as a plan and never as a completed deployment*.
The plan lane is the one writer that does not follow it.

The state is chosen in two places, from two private copies:

```rust
// src/cli/release.rs, before — the deploy plan
let state_label = if plan.healthy() { "done" } else { "blocked" };
```

```rust
// src/cli/publish.rs — the deploy apply
let state_label = report.verdict.journal_state();
```

The plan's copy has no plan arm at all, which is how a plan reached
`done`. The row also reaches the browser: the admin delivery timeline
renders every `deploy` operation row with its state, so the operator
reads a completed deployment that never happened.

This is the rule the repair programme is built on: Forge states
something its own gathered evidence does not support.

## What Changes

- **One rule names what a deploy run did**, in one place in the deploy
  model: the verdict type gains the plan's own arm, so the plan lane and
  the apply lane read the same vocabulary.
- **The plan's journal row follows that rule.** A ready plan records
  `planned` and a plan the shared delivery rule blocks records
  `blocked`. It never records `done`.
- **The detail names the run**, so a reader never has to guess what the
  state means.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The run rule | `forge` | Yes | `src/deploy/model.rs`: `DeployVerdict` gains the plan arm, and `verdict_for_plan(ready)` is the plan's one rule. |
| The deploy plan CLI | `forge` | Yes | `src/cli/release.rs`: the plan's journal row follows the rule. |
| The deploy apply CLI | `forge` | Read only | `src/cli/publish.rs` already reads `report.verdict.journal_state()`; the apply's states do not change. |
| The deploy contract test | `forge` | Added | `tests/deploy_plan_verdict_truth_contract.rs`. |
| The `deploy-plan-verdict-truth` capability | `forge` | Added | this package |
| The `adapter-deployment` capability | `forge` | Modified | the health requirement names the plan lane. |
| The deploy plan API route | `forge` | Read only | It is a read-only view, it journals nothing and its view carries `ready` and the delivery rule's reason. |
| The deploy state file | `forge` | Read only | A plan writes none. |
| The deploy contract version | `forge` | Read only | Unchanged. |
| The admin delivery timeline | `forge` | Read only | It renders the journal rows; the fix changes what it renders. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §30 owns the deployment lane; the `adapter-deployment` capability owns the deploy journal rule and already requires a run that deployed nothing to be journalled as a plan. |
| Contracts | No field is removed or renamed. `DEPLOY_CONTRACT_VERSION` is unchanged, the report's `verdict` field is unchanged, and the plan's JSON view is unchanged. The journal detail gains one field, `verdict=`. |
| Callers | `forge deploy plan`, its journal row and the admin delivery timeline that renders it. |
| Persistence | The journal state for a ready deploy plan becomes `planned`; a blocked plan keeps `blocked`; the apply lane keeps `done`, `planned` and `failed`. |
| Integrations | None. No adapter, provider or repository is touched. |
| Failures | No new error code. A plan still exits 0, ready or blocked. |
| Tests | A new contract test `tests/deploy_plan_verdict_truth_contract.rs`; the existing deploy contract, cross-surface and dry-run verdict tests stay green. |
| Dependencies | None added. |
| Concerns | Quality: the deploy journal can no longer be read as "a deploy happened" when only a plan was computed, and the browser timeline inherits the fix. Compatibility: a reader of the journal state for a deploy plan now reads `planned` instead of `done`; the detail gains a `verdict=` field. Security: no new surface; the redaction rule and the evidence handling are unchanged. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `deploy-plan-verdict-truth` | forge | Rust | added | this package |
| `adapter-deployment` | forge | Rust | modified | the health requirement names the plan lane. |

## Non-goals

- **The plan's output is unchanged.** The plan still prints its target,
  its artifact, its readiness and its note.
- **The apply lane's behaviour is unchanged.** A rehearsal still records
  `planned`, a delivered deploy `done` and a broken one `failed`.
- **The plan's JSON view is unchanged.** The admin plan route is
  read-only and journals nothing.
- **The deploy state file is not in this package.** A plan writes none.
- **The deploy contract version is unchanged.**
