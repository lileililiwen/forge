## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The fleet verdict rule | `forge` | Yes. One typed verdict, computed once from the entries. |
| The fleet report model | `forge` | Yes. `src/upgrade/model.rs`. |
| The fleet run | `forge` | Yes. `src/upgrade/engine.rs`. |
| The fleet human renderer | `forge` | Yes. `src/upgrade/engine.rs`. |
| The fleet JSON document and exit code | `forge` | Yes. `src/cli/feature.rs`. |
| The fleet verdict contract test | `forge` | Yes. `tests/upgrade_fleet_verdict_truth_contract.rs` (new). |
| The `upgrade-fleet-verdict-truth` capability | `forge` | Added. |
| The `project-upgrade-orchestration` capability | `forge` | Modified. |
| The single-project upgrade | `forge` | Read only. |
| The fleet registry surface | `forge` | Read only. |
| The upgrade plan | `forge` | Read only. |

## The Gap

### The rule never asked whether a project was judged

```rust
// src/upgrade/model.rs, before
impl FleetReport {
    /// A fleet is healthy only when no project failed or blocked.
    /// Skipped (already satisfied, unavailable, dry-run) is healthy.
    pub fn healthy(&self) -> bool {
        self.failed == 0 && self.blocked == 0
    }
}
```

The fleet's four per-project results were `success`, `failure`,
`blocked` and `skipped`. Two of the four `skipped` reasons mean the
project was never judged at all:

```rust
// src/upgrade/engine.rs, before
Err(_) => {
    entries.push(FleetEntry {
        project_id: project_id.clone(),
        status: "skipped".to_string(),
        changed: false,
        note: "skipped: project record unreadable at selection time".to_string(),
        ...
    });
    continue;
}
```

```rust
// src/upgrade/engine.rs, before
if !dir.is_dir() {
    entries.push(FleetEntry {
        project_id: project_id.clone(),
        status: "skipped".to_string(),
        changed: false,
        note: "skipped: project path unavailable; nothing was changed".to_string(),
        ...
    });
    continue;
}
```

Both counted towards `skipped`, and `skipped` counted as healthy. The
absence of a failure was read as the presence of a success.

### Three readers, no verdict in the document

The human renderer:

```rust
// src/upgrade/engine.rs, before
lines.push(format!(
    "verdict: {}",
    if report.healthy() {
        "healthy"
    } else {
        "not healthy"
    }
));
```

The exit code:

```rust
// src/cli/feature.rs, before
if report.healthy() {
    ExitCode::SUCCESS
} else {
    ExitCode::from(1)
}
```

And the JSON document — `serde_json::json!({"fleet": report})` — which
published the four counts and **no verdict at all**, so a reader of the
JSON had no way to tell a judged fleet from an unjudged one.

## The Rule

One typed verdict, computed once from the entries, read by every
surface.

```rust
/// The fleet's verdict: what the run established about the fleet.
pub enum FleetVerdict {
    /// Every selected project was judged, and none failed or blocked.
    Healthy,
    /// At least one selected project could not be judged.
    Unavailable,
    /// At least one selected project failed or blocked.
    NotHealthy,
}
```

The rule is one sentence:

> A fleet is healthy only when every selected project was judged and
> none failed or blocked. A project that could not be judged makes the
> verdict `unavailable`, never `healthy`. A project that failed or
> blocked makes it `not healthy`.

**What "judged" means.** A project is judged when the fleet read its
state and reached a conclusion about it: it was upgraded (`success`), it
failed (`failure`), it was blocked before any mutation (`blocked`), or
it was already satisfied (`skipped` — the plan is a no-op and the
fleet says so). A project is **not** judged when the fleet could not
read it at all: its registry record was unreadable, or its directory is
gone.

**The per-project status tells the truth.** A project that could not be
judged is reported as `unavailable`, a fifth result distinct from
`skipped`. The entry's note keeps the reason it gives today and its
label now names the result, so the operator reads *why* the project was
not judged and the line does not contradict itself.

**The verdict is computed once.** `run_fleet` computes the verdict from
the entries it produced and stores it on the report. The human
renderer, the JSON document and the exit code read that one value; no
surface re-derives it.

**The verdict is published.** The report carries the verdict as a
field, so the JSON document states it — a reader of the JSON can tell a
judged fleet from an unjudged one without counting.

**The exit code follows the verdict.** `healthy` exits 0; `unavailable`
and `not healthy` exit 1. An unavailable fleet upgraded nothing and
established nothing, so it must not report success to a script.

## Failure Boundaries

| Boundary | Behaviour |
|---|---|
| Every project upgraded | Verdict `healthy`; the exit code is 0. |
| Every project already satisfied | Verdict `healthy` — each project was judged. |
| A project failed | Verdict `not healthy`; the exit code is 1. |
| A project blocked | Verdict `not healthy`; the exit code is 1. |
| A project's path is unavailable | Verdict `unavailable`; the exit code is 1. |
| A project's record is unreadable | Verdict `unavailable`; the exit code is 1. |
| A project failed **and** another was unavailable | Verdict `not healthy` — the definite failure outranks the unknown. |
| A dry-run fleet | Unchanged: the header says `(dry-run)` and the plan was judged. A dry-run fleet with an unavailable project reports `unavailable`. |
| The registry holds no project | The selection is empty; the fleet has nothing to judge and reports `healthy`, as today. |

## Risks

- **The per-project `status` vocabulary gains `unavailable`.** A reader
  that matched `skipped` to mean "not judged" must now match
  `unavailable`. The reason text is unchanged, so an operator reading the
  output sees the same reason as before; only the label naming the
  result changed.
- **The journal rows are unchanged.** An unavailable project still
  records an `upgrade` operation row with the state `skipped`, because
  the operation changed nothing. The journal row and the fleet entry
  answer different questions: the row records what the operation did,
  the entry records what the fleet established.
- **The `skipped` count drops for an unavailable project.** It is
  replaced by the new `unavailable` count. A fleet of only unavailable
  projects reports `skipped: 0, unavailable: 1`.
- **The exit code for an unavailable fleet becomes 1.** A script that
  treated "the fleet could not judge anything" as success now sees a
  failure — which is what the verdict means.
- **The contract version is unchanged.** The report gains a field; no
  field is removed or renamed.
- **Nothing is written differently.** The journal rows, the manifests,
  the receipts and the plans are untouched.
