## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The run rule | `forge` | Yes. `src/release/model.rs`. |
| The release prepare | `forge` | Yes. `src/cli/release.rs`. |
| The release apply | `forge` | Yes. `src/cli/release.rs`. |
| The release contract test | `forge` | Added. `tests/release_plan_verdict_truth_contract.rs`. |
| The `release-plan-verdict-truth` capability | `forge` | Added. |
| The `release-publishing` capability | `forge` | Modified. |
| The `release-dry-run-evidence-truth` capability | `forge` | Modified. |
| The release plan API route | `forge` | Read only. |
| The release state file | `forge` | Read only. |
| The release contract version | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/cli/release.rs, before
let state_label = if plan.healthy() { "done" } else { "blocked" };
let _ = registry.record_operation("release", &project_id, state_label, &detail);
```

### The row it wrote

```
('release', 'done', 'release prepare `rel-probe` v1.0.0 revision=`8cee66eb27be9c8555f21e9b80e61abca0fc2e88` ready=true dry_run=false')
```

Nothing was released: no commit, no tag, no publication.

### The second copy of the rule

```rust
// src/cli/release.rs, before
let state_label = if report.dry_run {
    "planned"
} else if report.healthy() {
    "done"
} else {
    "partial"
};
```

Two writers, two private copies. The prepare's copy has no dry-run
arm, so a plan reached `done`.

## The Rule

One rule, in the release model:

```rust
// src/release/model.rs
/// What a release run did.
///
/// The journal state names this and nothing else: a plan is a plan,
/// and an applied release is the only thing that is `done`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseRun {
    /// A plan the checks allow.
    Planned,
    /// A plan a check blocks.
    Blocked,
    /// An applied release whose stages all completed.
    Delivered,
    /// An applied release that did not complete every stage.
    Partial,
}

impl ReleaseRun {
    /// The registry journal state this run records.
    pub fn journal_state(self) -> &'static str {
        match self {
            ReleaseRun::Planned => "planned",
            ReleaseRun::Blocked => "blocked",
            ReleaseRun::Delivered => "done",
            ReleaseRun::Partial => "partial",
        }
    }

    /// The label the journal detail publishes.
    pub fn label(self) -> &'static str {
        match self {
            ReleaseRun::Planned => "planned",
            ReleaseRun::Blocked => "blocked",
            ReleaseRun::Delivered => "delivered",
            ReleaseRun::Partial => "partial",
        }
    }
}
```

### Both writers read it

```rust
// src/cli/release.rs, cmd_release_prepare
let run = if plan.healthy() {
    ReleaseRun::Planned
} else {
    ReleaseRun::Blocked
};
let detail = format!(
    "release prepare `{}` v{} revision=`{}` ready={} verdict={} dry_run={}",
    project_id,
    plan.identity.version,
    plan.identity.source_revision,
    plan.healthy(),
    run.label(),
    dry_run
);
let _ = registry.record_operation("release", &project_id, run.journal_state(), &detail);
```

```rust
// src/cli/release.rs, cmd_release_apply
let run = if report.dry_run {
    ReleaseRun::Planned
} else if report.healthy() {
    ReleaseRun::Delivered
} else {
    ReleaseRun::Partial
};
let detail = format!(
    "release apply `{}` v{} revision=`{}` stages={} verdict={} dry_run={} retry={} healthy={}",
    report.project_id,
    report.identity.version,
    report.identity.source_revision,
    report.stage_outcomes.len(),
    run.label(),
    report.dry_run,
    report.retry,
    report.healthy
);
let _ = registry.record_operation("release", &project_id, run.journal_state(), &detail);
```

The apply lane's behaviour is unchanged: a dry run still records
`planned`, an applied release `done` and an incomplete one `partial`.
Only the prepare's state changes, and the detail of both rows gains
the run's label so a reader never has to guess what the state means.

### The options, decided

| Option for the preparation's journal row | Verdict |
|---|---|
| Keep `done` and document that "done" means "the operation finished" | Rejected. In the release journal `done` is the state an applied release writes after every stage completed. A preparation would be indistinguishable from a delivery, and the canonical `release-dry-run-evidence-truth` already requires the journal to record a plan as a plan. |
| `planned` for a ready plan, `blocked` for a plan a check blocks | **Chosen.** The lane already has `planned` — change 31 gave the dry run that state — and the state then names what the run did. |
| A new state `prepared` | Rejected. New vocabulary for a plan the lane already names `planned`; it adds a word, not a truth. |
| Stop journalling the preparation | Rejected. The journal is the product's only record that a plan was computed and with which checks; dropping the row loses evidence. |

### Why the blocked case keeps its state

A preparation whose checks fail is not a plan the release may take.
`blocked` says the release cannot proceed and names the failing check
in the plan's own note, which is exactly what the operator needs. The
change only removes the false claim: a plan that *can* proceed is not
a release that *did*.

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A preparation whose checks pass | the run is `planned`; the row records `planned` with `verdict=planned` in its detail. |
| A preparation whose checks fail | the run is `blocked`; the row records `blocked`, as before. |
| A preparation run as a dry run | the run is `planned`; the row records `planned` and the detail names the dry run. The prepare writes no release state either way. |
| An applied release whose stages complete | the run is `delivered`; the row records `done`, as before. |
| An applied release that did not complete every stage | the run is `partial`; the row records `partial`, as before. |
| A dry run of a release apply | the run is `planned`; the row records `planned`, as before. |
| An unconfirmed release apply | unchanged: the command refuses before any stage runs and writes no row. |
| The release plan API route | unchanged: it is a read-only view and journals nothing. |
| The release state file | unchanged: a prepare writes none. |
| The exit codes | unchanged: a prepare exits 0, a blocked plan exits 0 with its reason, an applied release exits 0 and an incomplete one exits 1. |

## Risks

| Risk | Mitigation |
|---|---|
| A reader of the release journal's `done` state for a preparation breaks. | The value was false: a preparation released nothing. The new state says what the run did, and the detail keeps naming the command and the readiness. |
| A reader counts `state='done'` rows to count releases. | That count was wrong before this change: it counted preparations as releases. It is right afterwards. |
| The detail's format changes. | The detail gains one `verdict=` field; every other field keeps its name and its order. No test asserted the detail before this change. |
| The apply lane's rows change. | Their states do not change at all; only the detail gains the `verdict=` field. |
| The release contract version is unchanged. | Nothing is removed or renamed in the report or the plan. |
