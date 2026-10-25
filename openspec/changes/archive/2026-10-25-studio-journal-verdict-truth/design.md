# Design: studio-journal-verdict-truth

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The Studio journal rule | `forge` | Added. `src/studio/state.rs` (`StudioAction`). |
| The Studio refinement writer | `forge` | Yes. `src/studio/state.rs`. |
| The Studio spec-save writer | `forge` | Yes. `src/studio/state.rs`. |
| The Studio preview writers | `forge` | Yes. `src/studio/preview.rs`. |
| The Studio contract test | `forge` | Added. `tests/studio_journal_verdict_truth_contract.rs`. |
| The `studio-journal-verdict-truth` capability | `forge` | Added. |
| The `site-studio-preview-refinement` capability | `forge` | Modified. |
| The CLI and the two routes | `forge` | Read only. |
| The session record | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/studio/state.rs, before — the refinement writer
record_journal(
    registry,
    StudioJournalKind::Refine,
    &session.project_id,
    "done",
    &detail,
)?;
```

### What it produced

```
('studio.refine', 'studio-probe', 'done', 'app_revision=r2 request=make the hero bigger selected_files=(none)')
```

### The evidence, and where it is not

| The fact | Where it lives |
|---|---|
| The request that was submitted | The `request=` field of the row's own detail. |
| The revision that was accepted | The `app_revision=` field of the row's own detail. |
| The files the request named | The `selected_files=` field of the row's own detail. |
| A refinement that ran | Nothing. The Core validates, journals and bumps `app_revision`; the agent-adapter runtime hook is a follow-up. |

### The rule that does not exist yet

Every Studio row's state was a string the writer invented:

| Writer | State it passed | Honest? |
|---|---|---|
| `save_spec` | `"done"` | Yes — the spec was saved. |
| `record_refinement` | `"done"` | **No** — only a request was recorded. |
| preview start (ready) | `PreviewState::Ready.label()` | Yes — the observed state. |
| preview start (failed) | `PreviewState::Failed.label()` | Yes — the observed state. |
| preview stop | `PreviewState::Stopped.label()` | Yes — the observed state. |

One writer was wrong and nothing said so. A second writer would have
been free to write anything.

## The Rule

**One rule names what a Studio journal row records: the action the
Studio Core performed.**

### The rule lives in the Studio Core

```rust
// src/studio/state.rs, after
/// What the Studio Core did, and therefore what its journal row
/// records. A row names the action the Core performed; it never
/// claims an action the Core did not perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioAction {
    /// The Core saved the validated spec.
    SpecSaved,
    /// The Core recorded a refinement request. No refinement ran:
    /// the agent-adapter runtime hook is a follow-up.
    RefinementRecorded,
    /// The Core observed the preview reach a state.
    PreviewObserved(PreviewState),
}

impl StudioAction {
    /// The journal state this action records.
    pub fn journal_state(self) -> &'static str {
        match self {
            StudioAction::SpecSaved => "done",
            StudioAction::RefinementRecorded => "recorded",
            StudioAction::PreviewObserved(state) => state.label(),
        }
    }
}
```

### Every writer reads it

```rust
// src/studio/state.rs, after — the refinement writer
record_journal(
    registry,
    StudioJournalKind::Refine,
    &session.project_id,
    StudioAction::RefinementRecorded.journal_state(),
    &detail,
)?;
```

```rust
// src/studio/state.rs, after — the spec-save writer
record_journal(
    registry,
    StudioJournalKind::SpecSave,
    project_id,
    StudioAction::SpecSaved.journal_state(),
    &detail,
)?;
```

```rust
// src/studio/preview.rs, after — the preview writers
record_journal(
    registry,
    StudioJournalKind::PreviewStart,
    &self.project_id,
    StudioAction::PreviewObserved(state).journal_state(),
    &detail,
)?;
```

The row after the change:

```
('studio.refine', 'studio-probe', 'recorded', 'app_revision=r2 request=make the hero bigger selected_files=(none)')
```

The three surfaces — the CLI, the operator route and the admin route —
all call `record_refinement`, so the one rule reaches all of them.

## The options, decided

| Option for the refinement row's state | Verdict |
|---|---|
| Keep `done` | Rejected. The refinement never ran; the Core's own doc comment and the CLI's own help say so. A row that claims a completion the Core never performed is the defect this programme keeps repairing. |
| Use `rejected` | Rejected. Nothing was refused. The request was accepted, validated and recorded; `rejected` would be a false failure. |
| Use `blocked` | Rejected. `blocked` is the registry's word for a rule that refused the action. Nothing refused this request. |
| Use `pending` or `queued` | Rejected. Nothing promises to run the request later. The agent-adapter runtime hook is a follow-up, not a queued job, and a row that says "queued" would be a promise the Core cannot keep. |
| Use `applied` | Rejected. No file was patched. The CLI's help says the path is read-only at the file level. |
| Use `recorded` | **Chosen.** It names exactly what happened: the request was validated and recorded, and nothing else. |
| Give every Studio writer its own state string | Rejected. That is the defect: one writer was free to write `done` for a request it never performed. |

| Option for where the rule lives | Verdict |
|---|---|
| A literal in the refinement writer | Rejected. A literal is a copy of the rule, and the copy is what drifted. |
| A rule in the Studio Core, read by every writer | **Chosen.** The Core owns the Studio's actions; the row's state comes from the action it performed, and no writer carries a hand-written state string. |
| A rule in the registry | Rejected. The registry stores the row; it does not know what the Studio did. The vocabulary is the Studio's, so the rule belongs to the Studio. |

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A refinement request that was accepted | The row is `recorded`, and the detail carries the request, the revision and the file list. |
| A refinement request with a stale revision | Unchanged: refused with `studio-revision-conflict`, and no row is written. |
| A refinement request with a path escape | Unchanged: refused with `studio-project-scope`, and no row is written. |
| A refinement request with a credential-shaped string | Unchanged: refused with `studio-invalid-spec`, and no row is written. |
| A saved spec | The row is `done`, unchanged. |
| A preview that became ready | The row is `ready`, unchanged. |
| A preview that failed to start | The row is `failed`, unchanged. |
| A stopped preview | The row is `stopped`, unchanged. |
| The session record | Unchanged, including the `app_revision` bump. |
| The CLI output and the two routes' envelopes | Unchanged. |

## Risks

| Risk | Mitigation |
|---|---|
| A reader counted `done` Studio rows to find completed work. | That count was wrong: it counted a recorded request as a performed refinement. The new state says which rows are requests. |
| A new Studio writer invents another state string. | The rule is in the Core and every current writer reads it. A future writer reads the same enum; the compiler points at it. |
| The preview rows change value. | They do not: `StudioAction::PreviewObserved(state).journal_state()` returns the same `PreviewState` label the preview writer already passed. The existing Studio preview contract tests pin those rows. |
