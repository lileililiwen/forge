# Design: api-agent-transition-verdict-truth

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The API agent transition writer | `forge` | Yes. `src/api/handlers_changes.rs`. |
| The agent journal rule | `forge` | Read only. `src/agent/model.rs`. |
| The API contract test | `forge` | Added. `tests/api_agent_transition_verdict_truth_contract.rs`. |
| The `agent-transition-verdict-truth` capability | `forge` | Modified. |
| The command and the MCP tool | `forge` | Read only. |
| The route's response and the session record | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/api/handlers_changes.rs, before
let detail = format!(
    "api agent `{id}` session `{session_id}` {transition} -> `{}`",
    outcome.state.label()
);
let _ = registry.record_operation("api", id, "done", &detail);
```

### What it produced

```
('api', 'api-agent', 'done', 'api agent `api-agent` session `s1` start -> `unsupported`')
```

The row says `done` and, in the same row, says `-> unsupported`. It
contradicts itself: the transition was refused, and the detail the
route itself wrote names the refusal.

### The evidence, and where it is not

| The fact | Where it lives |
|---|---|
| The transition the route was asked for | The `session` and `transition` fields of the row's own detail. |
| The state the runtime reported | The `-> unsupported` at the end of the row's own detail, and `outcome.state`. |
| The supervisor's verdict | Absent; the bundled path reports none. |
| A transition that ran | Nothing. The bundled provider exposes no backing runtime in this build, so the transition is refused. |

### The rule that already exists

```rust
// src/agent/model.rs
impl AgentTransitionOutcome {
    /// The journal state this outcome records.
    ///
    /// A row must never claim a completion the runtime did not report.
    pub fn journal_state(&self) -> String {
        match &self.verdict {
            Some(verdict) => verdict.clone(),
            None => match self.state {
                SessionState::Unsupported | SessionState::Disconnected => {
                    self.state.label().to_string()
                }
                _ => "done".to_string(),
            },
        }
    }
}
```

Change 44 put this rule in one place and promoted it, and named the two
writers that read it: "The command and the MCP tool SHALL read the
same rule." The HTTP route was never named, so it kept its own
literal — the drift the programme keeps repairing, one surface at a
time.

## The Rule

**Every surface that records an agent transition reads the one rule.**
No writer keeps a state of its own.

### The route reads the rule

```rust
// src/api/handlers_changes.rs, after
let detail = format!(
    "api agent `{id}` session `{session_id}` {transition} -> `{}`",
    outcome.state.label()
);
let _ = registry.record_operation("api", id, &outcome.journal_state(), &detail);
```

The row after the change:

```
('api', 'api-agent', 'unsupported', 'api agent `api-agent` session `s1` start -> `unsupported`')
```

The command's row for the same transition, unchanged:

```
('agent', 'api-agent', 'unsupported', '... start -> `unsupported`')
```

The two surfaces now agree.

### The requirement names every surface

```
Forge SHALL record an agent transition in the journal with the state the
runtime reported. ... Every surface that records an agent transition —
the command, the MCP tool and the HTTP route — SHALL read the same rule.
```

and the scenario:

```
#### Scenario: One rule, every agent writer

- **WHEN** the command, the MCP tool and the HTTP route record an agent
  transition
- **THEN** all read the one rule for the state the row records
```

## The options, decided

| Option for the route's row's state | Verdict |
|---|---|
| Keep `done` | Rejected. The runtime refused the transition, and the row's own detail says so. A row that claims a completion the runtime refused is the defect. |
| Use `outcome.state.label()` directly | Rejected. The rule exists and knows more: it reports the supervisor's verdict when one was reported. A second decision in the route would drift from the rule again. |
| Read `outcome.journal_state()` | **Chosen.** The route joins the rule; the state is the state the runtime reported, and a supervised transition keeps its verdict. |
| Leave the route alone and widen the requirement only | Rejected. A requirement that says a thing the code does not do is the same defect in another place. |

| Option for the requirement | Verdict |
|---|---|
| Add a new capability for the API surface | Rejected. The rule is not new; only a writer was missing. A new capability would own a rule that already has an owner. |
| Modify the promoted requirement to name every surface | **Chosen.** The requirement said "the command and the MCP tool"; the code now does what the widened requirement says. |
| Leave the requirement as it is | Rejected. It would keep naming two surfaces while three read the rule, and would keep saying the route is free to write `done`. |

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A transition the runtime refused | The row records `unsupported` (or `disconnected`), exactly as the command and the MCP tool record it. |
| A transition the runtime carried out | The row records `done`, unchanged. |
| A supervised run-spec with a verdict | The row records the supervisor's verdict, unchanged. |
| A request the route refuses (bad body, unknown provider, unknown transition, missing session) | Unchanged: the route answers `400` or `404` and writes no transition row. |
| The route's response | Unchanged: `202` with the transition's own state. |
| The `api.agent` operation row | Unchanged: the request's lifecycle, `pending` → `done`. The route's body validation runs before the lifecycle helper reserves the operation, so a request the route refuses (`400`) writes no operation row at all. |

### The two rows the route writes

| Row | Kind | State | What it records |
|---|---|---|---|
| The transition's row | `api` | the state the runtime reported | the transition and the state it reached |
| The operation's row | `api.agent` | `pending` → `done` | that the HTTP request was handled |

They are different facts and say so in their details: the transition's
row names the transition and the state it reached, the operation's row
says `api.agent completed`. The requirement names both, so no reader
has to guess which is which.
| The session record | Unchanged. |

## Risks

| Risk | Mitigation |
|---|---|
| A reader counted `done` API rows to find completed transitions. | That count was wrong: it counted refused transitions as completed ones. The new state says which rows are refusals. |
| The route's row and the operation row now disagree (`unsupported` and `done`). | They are different facts and say so: the operation row's detail is `api.agent completed` (the request was handled) and the transition row's detail names the transition and the state it reached. |
| The rule changes the route's behaviour somewhere else. | It does not: `journal_state()` is a pure function of the outcome, and the route's response body is built from the same outcome as before. |
