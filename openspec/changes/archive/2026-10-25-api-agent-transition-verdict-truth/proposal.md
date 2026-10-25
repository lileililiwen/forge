# Change: api-agent-transition-verdict-truth

## Why

The HTTP agent route journalled a refused transition as a completed
one, while the command and the MCP tool journaled the same transition
honestly.

```
$ forge --registry /tmp/api-agent/registry.db agent start /tmp/api-agent/proj \
    --session s1 --provider codex
{"transition":{"state":"unsupported", ...}}
# the command's row:  ('agent', 'api-agent', 'unsupported', '... start -> `unsupported`')

$ POST /v1/projects/api-agent/agents  {"session":"s1","transition":"start","provider":"codex"}
202 Accepted
{"agent":{"transition":{"state":"unsupported", ...}}, ...}
# the route's row:   ('api', 'api-agent', 'done', 'api agent `api-agent` session `s1` start -> `unsupported`')
```

The route's row said `done` — a completed action — for a transition the
runtime refused, and its own detail said `-> unsupported` in the very
same row. The row contradicted itself.

The state was a literal in the route:

```rust
// src/api/handlers_changes.rs, before
let detail = format!(
    "api agent `{id}` session `{session_id}` {transition} -> `{}`",
    outcome.state.label()
);
let _ = registry.record_operation("api", id, "done", &detail);
```

Change 44 put the rule in one place and promoted it:

```rust
// src/agent/model.rs
impl AgentTransitionOutcome {
    pub fn journal_state(&self) -> String { ... }
}
```

The command and the MCP tool read it. The HTTP route did not, and the
promoted requirement only named those two surfaces: "The command and
the MCP tool SHALL read the same rule." The route was free to keep
writing `done`.

This is the programme's rule class: Forge states something its own
gathered evidence does not support, and the repair is one shared rule
in one place, read by every surface.

## What Changes

- **The HTTP route reads the shared rule.** The route's journal row
  records the state the runtime reported, the same way the command and
  the MCP tool record it.
- **The requirement names every surface.** The promoted requirement
  now says that every surface which records an agent transition reads
  the one rule, and the "one rule, every agent writer" scenario names
  the route.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The API agent transition row | `forge` | Yes | `src/api/handlers_changes.rs`: the state comes from the shared rule. |
| The agent journal rule | `forge` | Read only | `src/agent/model.rs` (`AgentTransitionOutcome::journal_state()`) is unchanged; the route joins it. |
| The command and the MCP tool | `forge` | Read only | Unchanged; their rows already carry the reported state. |
| The route's response | `forge` | Read only | The `202` envelope, the transition body and the exit code are unchanged. |
| The route's operation row | `forge` | Read only | The `api.agent` operation row is the request's lifecycle (`pending` → `done`); it is not the transition's verdict and its detail says so. |
| The session record | `forge` | Read only | `.forge/agents/<session>/session.json` is unchanged. |
| The registry | `forge` | Read only | No schema change; one row's `state` column changes. |
| The planner and studio lanes | `forge` | Read only | Unchanged (changes 45 and 46 own them). |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §14 owns the agent runtime; the `agent-transition-verdict-truth` capability owns the journal state rule. |
| Contracts | No field is removed or renamed. One API journal row's `state` column carries the reported state instead of `done`. |
| Callers | `POST /v1/projects/{id}/agents`. |
| Persistence | The `api` journal row's state changes; the `api.agent` operation row, the session record and the receipts are unchanged. |
| Integrations | None. No adapter, provider or repository is touched. |
| Failures | No new error code. The route still answers `202` with the transition's own state. |
| Tests | A new contract test `tests/api_agent_transition_verdict_truth_contract.rs`; the existing agent tests stay green. |
| Dependencies | None added. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `agent-transition-verdict-truth` | forge | Rust | modified | the requirement now names every surface that records a transition. |

No new capability: the rule already exists and is already promoted.
This change makes the last writer read it.

## Non-goals

- **The rule is not changed.** `AgentTransitionOutcome::journal_state()`
  keeps its meaning: the supervisor's verdict when one was reported, the
  reported state when the runtime refused or lost the session, and
  `done` when the runtime carried the transition out.
- **The route's response is unchanged.** It still answers `202` with the
  transition's own state, and the envelope is untouched.
- **The operation row is unchanged.** The `api.agent` row tracks the
  HTTP request's lifecycle (`pending` → `done`), which is a different
  fact from the transition's verdict; its detail says which one it is.
- **The other API journal writers are untouched.** The API's own
  lifecycle helper still finalizes an operation as `done` when the
  handler returns; a route whose handler fails writes `failed`, which is
  what the helper means.
- **No journal row is rewritten.** Rows written before this change stay
  as they are.
