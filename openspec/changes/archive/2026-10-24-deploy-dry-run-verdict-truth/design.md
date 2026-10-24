## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The verdict rule | `forge` | Yes. `src/deploy/model.rs`. |
| The deploy apply | `forge` | Yes. `src/deploy/engine/stages.rs`. |
| The deploy CLI | `forge` | Yes. `src/cli/publish.rs`. |
| The API's deploy view | `forge` | Yes. `src/api/admin/deploy_views.rs`. |
| The deploy unit tests | `forge` | Yes. `src/deploy/engine/stages_tests.rs`. |
| The deploy contract test | `forge` | Added. `tests/deploy_dry_run_verdict_truth_contract.rs`. |
| The `deploy-dry-run-verdict-truth` capability | `forge` | Added. |
| The `adapter-deployment` capability | `forge` | Modified. |
| The adapter's stage outcomes | `forge` | Read only. |
| The deploy state file | `forge` | Read only. |
| The deploy contract version | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/deploy/engine/stages.rs, before
let note = if healthy {
    format!(
        "deploy applied to target `{}` and observed as `{}`",
        plan.target.name, observation.status
    )
} else {
    format!(
        "deploy did not complete cleanly: apply={} observe={}",
        apply_outcome.status, observe_outcome.status
    )
};
```

### The same run's journal row

```
('deploy', 'dep-probe', 'done', 'deploy `dep-probe-home-b6630d40797e` target=`home` adapter=`local` stages=apply=delivered,observe=running healthy=true dry_run=true')
```

and no deploy state at all:

```
$ ls /tmp/dep-probe/proj/.forge/deploy
ls: cannot access '/tmp/dep-probe/proj/.forge/deploy': No such file or directory
```

### The health verdict cannot say "planned"

```rust
// src/deploy/engine/stages.rs, before
let healthy = stages.iter().all(|s| {
    s.status == STATUS_DELIVERED
        || s.status == STATUS_SKIPPED
        || s.status == STATUS_DISABLED
        || s.status == STATUS_RUNNING
});
```

### The writers that follow it

```rust
// src/cli/publish.rs, before
let detail = forge::deploy::engine::journal_report(&report, report.healthy);
let state_label = if report.healthy() { "done" } else { "partial" };
let _ = registry.record_operation("deploy", &project_id, state_label, &detail);
```

```rust
// src/api/admin/deploy_views.rs, before
"dry_run": report.dry_run,
"healthy": report.healthy,
```

## The Rule

One rule, in the one place the report is built:

```rust
// src/deploy/model.rs
/// The deploy run's verdict.
///
/// The verdict is what the run did. A rehearsal is a plan, and a
/// plan is not a delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeployVerdict {
    /// The deploy ran and every stage completed.
    Delivered,
    /// No deploy was attempted. The report is a rehearsal.
    Planned,
    /// A stage did not complete.
    Failed,
}

impl DeployVerdict {
    /// The stable label the report, the JSON document and the journal
    /// detail publish.
    pub fn label(self) -> &'static str {
        match self {
            DeployVerdict::Delivered => "delivered",
            DeployVerdict::Planned => "planned",
            DeployVerdict::Failed => "failed",
        }
    }

    /// The registry journal state this verdict records.
    pub fn journal_state(self) -> &'static str {
        match self {
            DeployVerdict::Delivered => "done",
            DeployVerdict::Planned => "planned",
            DeployVerdict::Failed => "failed",
        }
    }
}

/// The one rule that judges a deploy run.
///
/// A rehearsal attempted no deploy, whatever its stages say. A run
/// that attempted a deploy is a delivery only when every stage
/// completed.
pub fn verdict_for_report(dry_run: bool, healthy: bool) -> DeployVerdict {
    if dry_run {
        return DeployVerdict::Planned;
    }
    if healthy {
        DeployVerdict::Delivered
    } else {
        DeployVerdict::Failed
    }
}
```

The report carries the verdict, and its health is derived from it:

```rust
// src/deploy/engine/stages.rs
let healthy = stages.iter().all(|s| {
    s.status == STATUS_DELIVERED
        || s.status == STATUS_SKIPPED
        || s.status == STATUS_DISABLED
        || s.status == STATUS_RUNNING
});
let verdict = DeployVerdict::verdict_for_report(request.dry_run, healthy);
let healthy = verdict == DeployVerdict::Delivered;
```

### The note follows the verdict

```rust
let note = match verdict {
    DeployVerdict::Planned => format!(
        "dry-run: {} stage(s) rehearsed; nothing was deployed",
        stages.len()
    ),
    DeployVerdict::Delivered => format!(
        "deploy applied to target `{}` and observed as `{}`",
        plan.target.name, observation.status
    ),
    DeployVerdict::Failed => format!(
        "deploy did not complete cleanly: apply={} observe={}",
        apply_outcome.status, observe_outcome.status
    ),
};
```

### The journal row follows the verdict

```rust
// src/cli/publish.rs
let detail = forge::deploy::engine::journal_report(&report);
let _ = registry.record_operation(
    "deploy",
    &project_id,
    report.verdict.journal_state(),
    &detail,
);
```

and the detail names it:

```rust
// src/deploy/engine/stages.rs
pub fn journal_report(report: &DeployReport) -> String {
    ...
    format!(
        "deploy `{}` target=`{}` adapter=`{}` stages={} verdict={} healthy={} dry_run={}",
        report.identity.id,
        report.target.name,
        report.adapter,
        stage_summary,
        report.verdict.label(),
        report.healthy,
        report.dry_run
    )
}
```

### The exit code follows the verdict

```rust
// src/cli/publish.rs
match report.verdict {
    DeployVerdict::Delivered | DeployVerdict::Planned => Ok(output),
    DeployVerdict::Failed => {
        render_output(output);
        Err(ForgeError::DeployHealthFailed { reason: report.note.clone() })
    }
}
```

A rehearsal is a successful rehearsal: it exits 0, exactly as it
does today. A failed deploy still exits 1 with the report on stdout.

### What each surface does with the verdict

| Surface | Before | After |
|---|---|---|
| the report's note (a rehearsal) | "deploy applied to target `X` and observed as `Y`" | "dry-run: N stage(s) rehearsed; nothing was deployed" |
| the report's `healthy` (a rehearsal) | `true` | `false` |
| the report's new `verdict` field | absent | `delivered`, `planned` or `failed` |
| the journal state (a rehearsal) | `done` | `planned` |
| the journal detail | `healthy={}` | `verdict={} healthy={}` |
| the exit code (a rehearsal) | 0 | 0 (unchanged) |
| the API's deploy view | `healthy: true` | `verdict: "planned"`, `healthy: false` |

### Why the adapter's stage outcomes do not change

The canonical contract says Forge "records the stage outcome exactly
as the envelope names it and attributes the evidence to the adapter
and its revision". The stage statuses and the stage notes are the
adapter's envelope, and the adapter owns that claim. What Forge owns
is the verdict, the note, the health and the journal row: those are
the words Forge writes about its own run, and they are the words that
were false.

### Why the API view carries the verdict

The browser reads the same report as the CLI. A view that shows
`dry_run: true` next to `healthy: true` makes the same two answers
to one run that the journal made. The view carries the verdict, so
the browser reads the one rule.

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A rehearsal of a healthy adapter | the verdict is `planned`; the note says the stages were rehearsed and nothing was deployed; the report is not healthy; the journal records `planned`; the exit code is 0. |
| A rehearsal of a failing adapter | the verdict is `planned`; the note says the stages were rehearsed and nothing was deployed; the report is not healthy; the journal records `planned`; the exit code is 0. A rehearsal never fails the command. |
| A real deploy whose stages complete | the verdict is `delivered`; the note is unchanged; the report is healthy; the journal records `done`; the exit code is 0. |
| A real deploy whose stages do not complete | the verdict is `failed`; the note is unchanged; the report is not healthy; the journal records `failed`; the exit code is 1 with `DeployHealthFailed` and the report on stdout. |
| The deploy state file | a rehearsal writes none, exactly as today; a real deploy writes one, exactly as today. |
| The ship-gate refusal | unchanged: a rehearsal is never refused by the delivery rule. |
| The adapter invocation | unchanged: the same argv, the same `dry_run` flag in the payload, the same evidence redaction. |
| The stage outcomes | unchanged: the adapter's envelope is recorded exactly as it names it. |

## Risks

| Risk | Mitigation |
|---|---|
| A reader of the report's `healthy` breaks. | The value was true because the adapter rehearsed successfully. A rehearsal is not evidence that anything is running, so the value is now false and the verdict says why. |
| A reader of the journal state `done` for a rehearsal breaks. | The state becomes `planned` and the detail names the verdict and the dry run, so the row stays unambiguous. |
| A rehearsal now looks like a failure. | The exit code follows the verdict, not the health: a rehearsal exits 0 with the report on stdout, exactly as today. |
| The stage statuses still say `delivered` in a rehearsal. | They are the adapter's envelope, recorded exactly as the canonical contract requires. Forge's own words — the verdict, the note, the health and the journal row — are what this change makes true. |
| The report's JSON gains a field. | Nothing is removed or renamed; the deploy contract version stays. |
