# Design: agent-transition-verdict-truth

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The agent outcome | `forge` | Yes. `src/agent/model.rs`. |
| The CLI writers | `forge` | Yes. `src/cli/agent.rs`. |
| The MCP writers | `forge` | Yes. `src/mcp/methods.rs`. |
| The agent contract test | `forge` | Added. `tests/agent_transition_verdict_truth_contract.rs`. |
| The agent contract test | `forge` | Modified. `tests/agent_contract.rs`. |
| The `agent-transition-verdict-truth` capability | `forge` | Added. |
| The `agent-runtime-workflows` capability | `forge` | Modified. |
| The session record | `forge` | Read only. |
| The supervisor path | `forge` | Read only. |
| The other lanes | `forge` | Read only. |
| The portal's activity view | `forge` | Read only. |

## The Gap

### The claims

```rust
// src/cli/agent.rs, before — the run-spec writer
let verdict = outcome
    .verdict
    .clone()
    .unwrap_or_else(|| "done".to_string());
let detail = format!(
    "agent `{}` run_spec session `{}` -> `{}` (verdict `{verdict}`)",
    project_id,
    session,
    outcome.state.label()
);
let _ = registry.record_operation("agent", &project_id, &verdict, &detail);
```

```rust
// src/cli/agent.rs, before — the transition writer
let _ = registry.record_operation("agent", project_id, "done", &detail);
```

```rust
// src/mcp/methods.rs, before — the run-spec writer
let verdict = outcome.verdict.clone().unwrap_or_else(|| "done".to_string());
let _ = registry.record_operation("mcp", project_id, &verdict, &detail);
```

```rust
// src/mcp/methods.rs, before — the transition writer
let _ = registry.record_operation("mcp", &project_id, "done", &detail);
```

### What they produced

```
('agent', 'agent-probe', 'done', 'agent `agent-probe` run_spec session `s1` -> `unsupported` (verdict `done`)')
('agent', 'agent-probe', 'done', 'agent `agent-probe` session `s1` provider `codex` spec `` -> `unsupported`')
```

### Where the evidence is, and where it is not

| The fact | Where it lives |
|---|---|
| The state the runtime reported | `outcome.state` — `idle`, `active`, `paused`, `disconnected`, `unsupported`, `stopped`. |
| The run-spec verdict, when a supervisor reported one | `outcome.verdict` — `done`, `partial`, `unverified`. |
| The verdict when no supervisor ran | Nowhere. The bundled path returns the outcome of a plain `start` transition, whose `verdict` is `None`. |

The run-spec envelope agrees: the JSON document has no `verdict` key at
all, because `AgentTransitionOutcome.verdict` is skipped when it is
`None`.

```rust
// src/agent/model.rs
#[serde(skip_serializing_if = "Option::is_none")]
pub verdict: Option<String>,
```

So on the bundled path the only place the word `done` appears is the
journal row and the `registry_detail` string that goes with it.

### The canonical rule

> Forge SHALL expose only transitions the backing runtime genuinely
> supports; unsupported requests SHALL report the operator's real
> control path (attach guidance or the runtime's own pause primitive)
> and SHALL never claim a state the runtime did not report.

> WHEN the supervisor exits clean with verification passed THEN the
> journaled run-spec verdict is done with the supervisor named as the
> evidence source.

`done` is the verdict for a *verified* supervisor run. The bundled
adapter refused the transition, reported `unsupported` with its reason
and its next step, and produced no verdict at all.

## The Rule

**One rule decides the journal state: a run-spec records the
supervisor's verdict when the supervisor reported one, a transition the
runtime refused or lost is recorded as the state it reported, and a
transition the runtime carried out is recorded as `done`. A refused
transition is never `done`.**

### One rule, in the agent module

```rust
// src/agent/model.rs
impl AgentTransitionOutcome {
    /// The journal state this outcome records.
    ///
    /// A row must never claim a completion the runtime did not report:
    /// a run-spec records the supervisor's verdict when the supervisor
    /// reported one, and a transition the runtime refused or lost is
    /// journalled as the state it reported, never as `done`.
    pub fn journal_state(&self) -> String {
        match &self.verdict {
            Some(verdict) => verdict.clone(),
            None => match self.state {
                // The runtime refused the transition or lost the
                // session: the row names the state it reported.
                SessionState::Unsupported | SessionState::Disconnected => {
                    self.state.label().to_string()
                }
                // The runtime carried out the transition.
                _ => "done".to_string(),
            },
        }
    }
}
```

The agent module owns the session states and the verdict vocabulary, so
this is the one place that decides what a row may claim. The CLI and
the MCP tool both read it, so the two surfaces cannot drift apart.

### The writers read it

```rust
// src/cli/agent.rs, after — the run-spec writer
let verdict = outcome
    .verdict
    .clone()
    .unwrap_or_else(|| "none reported".to_string());
let detail = format!(
    "agent `{}` run_spec session `{}` -> `{}` (verdict `{verdict}`)",
    project_id,
    session,
    outcome.state.label()
);
let _ = registry.record_operation("agent", &project_id, &outcome.journal_state(), &detail);
```

```rust
// src/cli/agent.rs, after — the transition writer
let _ = registry.record_operation("agent", project_id, &outcome.journal_state(), &detail);
```

### The rows after the change

```
('agent', 'agent-probe', 'unsupported', 'agent `agent-probe` run_spec session `s1` -> `unsupported` (verdict `none reported`)')
('agent', 'agent-probe', 'unsupported', 'agent `agent-probe` session `s1` provider `codex` spec `` -> `unsupported`')
```

Each row now agrees with itself: the state is the outcome the detail
names.

The supervised path is unchanged:

```
('agent', 'agent-probe', 'done', 'agent `agent-probe` run_spec session `s1` -> `active` (verdict `done`)')
```

## The options, decided

| Option for the journal state of a refused transition | Verdict |
|---|---|
| Keep `done` | Rejected. The row contradicts itself and claims a completion the runtime refused, against the canonical "SHALL never claim a state the runtime did not report". |
| Keep `done` and add "unsupported" to the detail | Rejected. The state is what a reader counts and filters on; a refusal behind a `done` state is still a completed operation on the record. |
| Name the reported state for every transition, including a successful one | Rejected for the successful case: the repository's own cross-surface test reads a completed delegated transition as `done`, and the state the runtime reported is already named in the row's detail. Naming it twice in two vocabularies would blur the operation's completion with the session's state. The refused case is different and is what this change repairs. |
| Write the reported state when the runtime refused or lost the session, and keep `done` for a transition the runtime carried out | **Chosen.** The refused row names the state the runtime reported, so the row agrees with its own detail; the carried-out row keeps the vocabulary the repository already reads. |
| Write a separate state word such as `blocked` | Rejected. `blocked` is the registry's word for a rule that refused the action; the runtime refused the *transition* and named the state `unsupported`. A second vocabulary for the same fact is a second copy of the rule. |
| Let each writer decide | Rejected. Two writers already disagreed with the runtime; the decision belongs to the module that owns the states and the verdicts. |
| Invent a `unverified` verdict for the bundled path | Rejected. `unverified` is the verdict the supervisor path produces when its outcome document cannot be parsed — a real observation of a failed verification. The bundled path observed no verification attempt at all; the state it reported is `unsupported`. |
| Refuse the run-spec for a provider with no backing runtime | Rejected. The canonical requirement says every requested operation "SHALL remain exposed and answerable for that provider": the honest answer is the `unsupported` state with its reason and next step, which the command already gives. |
| Rewrite the rows written before this change | Rejected. Nothing can know what an old row described. The rule applies to new rows. |

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A run-spec delegated to the supervisor, which verified the loop | The row is `done` with the supervisor's verdict in the detail — unchanged. |
| A run-spec delegated to the supervisor, which reported a retryable iteration | The row is `partial` — unchanged. |
| A run-spec delegated to the supervisor, whose outcome document could not be parsed | The row is `unverified` — unchanged. |
| A run-spec with no supervisor (the bundled provider) | The row is `unsupported`, and the detail says the runtime reported no verdict. |
| A transition the bundled adapter refused | The row is `unsupported`, with the runtime's reason in the detail. |
| A transition the supervised runtime carried out | The row is `done` — unchanged — and the detail names the state the runtime reported. |
| A session handle the runtime no longer knows | Unchanged: the command surfaces the lost state and keeps the session file. |
| The command's envelope | Unchanged: the state, the evidence, the note, the next step and the optional `verdict` key. |
| The session record | Unchanged: it already records the reported state. |
| The other lanes' journal states | Unchanged. |

## Risks

| Risk | Mitigation |
|---|---|
| A reader that counted `done` rows for the agent kind now counts the reported outcome. | That count was wrong: it counted refused transitions as completed ones. The supervised path still counts `done` exactly as before. |
| The journal's `state` column now carries `unsupported` or `disconnected` for a refused agent transition. | The column is free text and the portal prints it verbatim. The value is the outcome the row's detail names, so the row stays readable on its own. |
| Two writers could drift apart again. | Both read `AgentTransitionOutcome::journal_state()`, the one method that owns the decision. |
| A supervised transition that succeeded stays `done`. | Unchanged: the repository's own cross-surface test reads that row as `done`, and the row's detail names the state the runtime reported. The change touches only the rows that claimed a completion the runtime refused. |
