# Change: agent-transition-verdict-truth

## Why

The agent lane journals `done` for transitions the runtime refused.

`forge agent` and the `run_agent` MCP tool write one journal row per
session transition. Every row says `state=done` — including the rows
whose own detail says the session ended `unsupported`.

Reproduced on the current tree, in a project whose provider has no
backing runtime in this build:

```
$ forge --registry /tmp/agent-probe/registry.db agent run-spec /tmp/agent-probe --session s1
{
  "transition": {
    "state": "unsupported",
    "note": "transition `start` is unsupported on provider `codex`; session `s1` was `idle` and is recorded as `unsupported`",
    "registry_detail": "agent `agent-probe` run_spec session `s1` -> `unsupported` (verdict `done`)"
  }
}
```

and the journal rows it wrote:

```
('agent', 'agent-probe', 'done', 'agent `agent-probe` run_spec session `s1` -> `unsupported` (verdict `done`)')
('agent', 'agent-probe', 'done', 'agent `agent-probe` session `s1` provider `codex` spec `` -> `unsupported`')
```

Each row contradicts itself: the state says the operation completed and
the detail says the runtime refused it.

Two writes produce the false `done`:

```rust
// src/cli/agent.rs, before — the run-spec writer
let verdict = outcome
    .verdict
    .clone()
    .unwrap_or_else(|| "done".to_string());
let _ = registry.record_operation("agent", &project_id, &verdict, &detail);
```

```rust
// src/cli/agent.rs, before — the transition writer
let _ = registry.record_operation("agent", project_id, "done", &detail);
```

```rust
// src/mcp/methods.rs, before — the same two writes
let verdict = outcome.verdict.clone().unwrap_or_else(|| "done".to_string());
let _ = registry.record_operation("mcp", project_id, &verdict, &detail);
let _ = registry.record_operation("mcp", &project_id, "done", &detail);
```

The first default invents a verdict: the run-spec outcome carries
`verdict: None` when no supervisor ran, and the JSON envelope omits the
`verdict` key entirely (`skip_serializing_if = "Option::is_none"`), so
the `done` exists only in the journal row. The second write claims a
completion for every transition, whatever the runtime reported.

The canonical capability already forbids this:

> Forge SHALL expose only transitions the backing runtime genuinely
> supports; unsupported requests SHALL report the operator's real
> control path (attach guidance or the runtime's own pause primitive)
> and SHALL never claim a state the runtime did not report.

and, for the run-spec verdict:

> WHEN the supervisor exits clean with verification passed THEN the
> journaled run-spec verdict is done with the supervisor named as the
> evidence source.

`done` is the verdict for a *verified* supervisor run. It is not the
verdict for a transition the bundled adapter refused.

This is the programme's rule class: Forge states something its own
gathered evidence does not support.

## What Changes

- **One rule decides the journal state.** In the agent module, read by
  the CLI and by the MCP tool: a run-spec records the supervisor's
  verdict when the supervisor reported one, a transition the runtime
  refused or lost is recorded as the state the runtime reported, and a
  transition the runtime carried out is recorded as `done`.
- **A refused transition is never `done`.** A bundled-provider
  transition is journalled `unsupported`, the state the runtime
  reported, with the reason the runtime gave.
- **The run-spec detail names the verdict it saw.** A supervisor
  verdict is recorded as it is; a run with no verdict says so instead
  of claiming `done`.
- **The canonical requirement states the rule.** The
  `agent-runtime-workflows` capability says the journaled state names
  the reported outcome.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The agent outcome | `forge` | Yes | `src/agent/model.rs`: the outcome knows the journal state it records. |
| The CLI `start` and `new-session` writers | `forge` | Yes | `src/cli/agent.rs`: the row records the reported outcome. |
| The CLI run-spec writer | `forge` | Yes | `src/cli/agent.rs`: the row records the reported outcome. |
| The CLI transition writer | `forge` | Yes | `src/cli/agent.rs`: the row records the reported outcome. |
| The MCP run-spec writer | `forge` | Yes | `src/mcp/methods.rs`: the row records the reported outcome. |
| The MCP transition writer | `forge` | Yes | `src/mcp/methods.rs`: the row records the reported outcome. |
| The agent contract test | `forge` | Modified | `tests/agent_contract.rs`: the journal assertions follow the new rule. |
| The agent contract test | `forge` | Added | `tests/agent_transition_verdict_truth_contract.rs`. |
| The `agent-transition-verdict-truth` capability | `forge` | Added | this package |
| The `agent-runtime-workflows` capability | `forge` | Modified | the transition mapping and the run-spec verdict requirements state the general rule. |
| The session record | `forge` | Read only | The session file already records the state the runtime reported; this change does not touch it. |
| The supervisor path | `forge` | Read only | `classify_sisyphusfy_output` still yields `done`, `partial` and `unverified`; the row records that verdict unchanged. |
| The other lanes | `forge` | Read only | No other lane's journal state changes. |
| The portal's activity view | `forge` | Read only | It prints the journal state verbatim; `unsupported` reads as what it is. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §45-§47 own the supervised agent runtime; the `agent-runtime-workflows` capability owns the transition mapping and the run-spec verdict. |
| Contracts | No field is removed or renamed. The journal row's `state` column carries the reported outcome instead of a fixed word; the run-spec detail's verdict segment names the verdict it saw. |
| Callers | `forge agent start/pause/takeover/resume/restart/new-session/run-spec` and the `run_agent` MCP tool. |
| Persistence | The agent and MCP journal rows for a refused transition now read `unsupported` instead of `done`. |
| Integrations | None. No adapter, provider or repository is touched. |
| Failures | No new error code and no new refusal. A transition the runtime refuses still answers with its honest state and next step; only the journal row changes. |
| Tests | A new contract test `tests/agent_transition_verdict_truth_contract.rs`; the existing agent tests keep the supervised path green. |
| Dependencies | None added. |
| Concerns | Quality: the operations table stops claiming completed agent runs that never happened. Compatibility: the supervised path still journals `done`, `partial` and `unverified` exactly as before. Security: no new surface. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `agent-transition-verdict-truth` | forge | Rust | added | this package |
| `agent-runtime-workflows` | forge | Rust | modified | two requirements state the general rule. |

## Non-goals

- **The session record is unchanged.** It already records the state the
  runtime reported.
- **The supervisor path is unchanged.** A verified supervisor run is
  still `done`, a retryable one `partial`, an unparsable one
  `unverified`.
- **The command's own output is unchanged.** The envelope still
  reports the state, the evidence, the note and the next step; the
  `verdict` key still appears only for a delegated run.
- **The other lanes are unchanged.** Only the agent lane's journal
  state changes.
- **No journal row is rewritten.** Rows written before this change stay
  as they are.
