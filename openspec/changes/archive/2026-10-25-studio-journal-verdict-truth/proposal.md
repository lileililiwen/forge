# Change: studio-journal-verdict-truth

## Why

The Studio lane journals a row that claims a refinement was performed
when only a request was recorded.

```
$ forge --registry /tmp/studio-probe/registry.db studio refine studio-probe \
    --expected-revision r1 --request "make the hero bigger"
{
  "contract": "forge-studio-preview/0.1.0",
  "revision": "r2",
  "state": "none"
}

$ sqlite3 /tmp/studio-probe/registry.db \
    "SELECT kind, project_id, state, detail FROM operations WHERE kind = 'studio.refine'"
('studio.refine', 'studio-probe', 'done', 'app_revision=r2 request=make the hero bigger selected_files=(none)')
```

The row says `done` — a completed action — although no refinement ran.
The Core's own documentation says so, in two places:

```rust
// src/studio/state.rs, before
/// Record a refinement request. The journal row is the only
/// durable artefact in this cycle: the agent-adapter runtime
/// hook is a follow-up (see `tasks.md` §5.3). The validation is
/// bounded and read-only; the new `app_revision` is derived from
/// the session's current `app_revision`.
pub fn record_refinement(...) -> Result<StudioSession, ForgeError> {
```

and the CLI's own help text for the command:

```
/// Submit a refinement request. Validates the request,
/// journals a `studio.refine` row, and bumps `app_revision`.
/// Read-only at the file level — no editor patch is applied
/// in this cycle (the agent-adapter hook is a follow-up).
```

So the row contradicts the code and the help that describe it: the
request was validated, recorded and journaled, and nothing else
happened.

This is the programme's rule class: Forge states something its own
gathered evidence does not support. The same shape was repaired in the
agent lane (change 44) and in the planner lane (change 45).

The state was written as a literal in the Core:

```rust
// src/studio/state.rs, before
record_journal(
    registry,
    StudioJournalKind::Refine,
    &session.project_id,
    "done",
    &detail,
)?;
```

The Studio lane has no rule that says what a row records. Each writer
hands `record_journal` a string it invented, so the vocabulary is
whatever the last writer typed — the same drift the programme keeps
repairing, one lane at a time.

## What Changes

- **One rule names what a Studio journal row records.** The Studio
  Core owns the action it performed, and the row's state comes from
  that action. A recorded refinement request is `recorded`, never
  `done`; a saved spec stays `done`; a preview row keeps the observed
  preview state.
- **Every Studio writer reads that rule.** The spec-save writer, the
  refinement writer and the preview writers all take their state from
  the one rule, so no row carries a hand-written state string.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The Studio refinement row | `forge` | Yes | `src/studio/state.rs`: the row names the recorded request. |
| The Studio spec-save row | `forge` | Yes | `src/studio/state.rs`: unchanged in value, now read from the rule. |
| The Studio preview rows | `forge` | Yes | `src/studio/preview.rs`: unchanged in value, now read from the rule. |
| The `forge studio refine` CLI | `forge` | Read only | The command's output and its exit code are unchanged. |
| `POST /v1/projects/{id}/studio/refine` | `forge` | Read only | The route's response is unchanged. |
| `POST /v1/admin/projects/{id}/studio/refine-admin` | `forge` | Read only | The route's response is unchanged. |
| The session record | `forge` | Read only | `.forge/studio/session.json` is unchanged, including the `app_revision` bump. |
| The preview lifecycle | `forge` | Read only | The preview start/stop rows keep the observed preview state. |
| The planner lane | `forge` | Read only | Unchanged by this change (change 45 owns it). |
| The agent lane | `forge` | Read only | Unchanged by this change (change 44 owns it). |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §20 owns the Site Studio; the `site-studio-preview-refinement` capability owns the refinement cycle and the journal rows. |
| Contracts | No field is removed or renamed. One Studio journal row's `state` column carries `recorded` instead of `done`; the other rows keep their states. |
| Callers | `forge studio refine`, `POST /v1/projects/{id}/studio/refine`, `POST /v1/admin/projects/{id}/studio/refine-admin`. |
| Persistence | The `studio.refine` journal row reads `recorded` instead of `done`; the session record and the receipts are unchanged. |
| Integrations | None. No adapter, provider or repository is touched. |
| Failures | No new error code and no new refusal. A stale revision, a path escape and a credential-shaped request still refuse exactly as before. |
| Tests | A new contract test `tests/studio_journal_verdict_truth_contract.rs`; the existing Studio tests stay green. |
| Dependencies | None added. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `studio-journal-verdict-truth` | forge | Rust | added | the rule that names what a Studio journal row records. |
| `site-studio-preview-refinement` | forge | Rust | modified | the refinement requirement states what the row records. |

The rule is new: the Studio lane had no rule at all, so every writer
invented a state string. This change introduces it in one place.

## Non-goals

- **The refinement is not implemented.** The agent-adapter runtime hook
  stays a follow-up. This change makes the journal row tell the truth
  about what the Core did; it does not make the Core do more.
- **The session record is unchanged.** The `app_revision` bump, the
  persisted session and the preview state are untouched.
- **The responses are unchanged.** The CLI output, the two routes'
  envelopes and their exit codes stay as they are.
- **The spec-save and preview rows keep their states.** A saved spec is
  still `done`; a preview row still names the observed preview state.
- **No journal row is rewritten.** Rows written before this change stay
  as they are.
