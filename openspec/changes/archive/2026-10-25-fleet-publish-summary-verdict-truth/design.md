# Design: fleet-publish-summary-verdict-truth

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The fleet summary counters | `forge` | Yes. `src/cli/fleet.rs`. |
| The fleet summary document | `forge` | Yes. `src/cli/fleet.rs`. |
| The fleet contract test | `forge` | Added. `tests/fleet_publish_summary_verdict_truth_contract.rs`. |
| The `publish-fleet-dry-run-verdict-truth` capability | `forge` | Modified. |
| The per-project rows, the journal rows, the fail-fast behaviour | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/cli/fleet.rs, before
// A plan is not a failure: the fleet counts it
// as neither, and it never trips `--fail-fast`.
if verdict == PublishVerdict::Failed {
    failure_count += 1;
    if fail_fast && first_failure.is_none() { ... }
} else {
    success_count += 1;
}
```

### What it produced

```
fleet publish: 1/1 publishable succeeded, 0 held back, 0 skipped (1 in inventory:...)
```

```json
"fleet": {"publishable": 1, "success": 1, "failed": 0,
          "projects": [{"project": "alpha", "healthy": false, "verdict": "planned"}]}
```

### The evidence, and where it is not

| The fact | Where it lives |
|---|---|
| The run was a plan | The lane's own per-project row: `verdict: planned`, `healthy: false`. |
| The run published nothing | The journal row's state: `planned`. |
| A delivery | Nothing. The dry run contacted nobody. |

The summary claimed a delivery the lane's own rows deny.

### The rule that already exists

The lane already judges every run with the one publish rule:

```rust
// src/cli/fleet.rs
/// The fleet lane's per-project verdict, read from the project's own
/// publish reports. A project whose reports are all plans is a plan, a
/// project with a failed report is a failure, and a project that
/// delivered is a delivery. One rule, shared by the fleet lanes.
fn fleet_project_verdict(...) -> PublishVerdict
```

and the promoted capability says:

> Forge SHALL judge a fleet provider publish with the one publish rule: a
> run that attempted no write is a plan, a run the provider reported
> healthy is a delivery, and a run the provider did not report healthy is
> a failure. ... **A plan SHALL NOT be counted as a failure.**

The summary's counters never read that rule. They counted a binary.

## The Rule

**The fleet summary counts the verdict: a delivery is a success, a plan
is a plan, a failure is a failure.**

### The rule lives in the fleet lane, next to the verdict rule

```rust
// src/cli/fleet.rs, after
/// The fleet summary's counters: what the runs did.
///
/// The counters come from the run's verdict, never from a binary: a
/// delivery is a success, a plan is a plan and a failure is a failure.
/// A plan is never counted as a success.
#[derive(Debug, Default, Clone, Copy)]
struct FleetCounters {
    success: usize,
    planned: usize,
    failed: usize,
}

impl FleetCounters {
    /// Count one run's verdict.
    fn record(&mut self, verdict: PublishVerdict) {
        match verdict {
            PublishVerdict::Delivered => self.success += 1,
            PublishVerdict::Planned => self.planned += 1,
            PublishVerdict::Failed => self.failed += 1,
        }
    }

    /// Count a run that failed before any verdict was reached.
    fn record_failure(&mut self) {
        self.failed += 1;
    }
}
```

### Every fleet lane counts its verdict

The three lanes that own a verdict — the parallel lane, the provider
lane and the adapter lane — read the same counters:

```rust
// src/cli/fleet.rs, after
counters.record(verdict);
if verdict == PublishVerdict::Failed && fail_fast && first_failure.is_none() {
    first_failure = Some(ForgeError::PublishDeployFailed {
        reason: format!("fleet publish failed at `{project_id}`"),
    });
    break;
}
```

The parallel lane gains the delivery it was missing: it had no `else`
branch, so a delivery was counted as nothing.

### The summary names the plans

```rust
// src/cli/fleet.rs, after
"success": counters.success,
"planned": counters.planned,
"failed": counters.failed,
```

```rust
// src/cli/fleet.rs, after
format!(
    "fleet publish: {}/{} publishable succeeded, {} planned, {} held back, {} skipped \
     ({} in {source_label}){}",
    counters.success,
    eligible.len(),
    counters.planned,
    held_back.len(),
    skipped.len(),
    snapshot.declared_count(),
    shared_host_note(held_back.len()),
)
```

## The options, decided

| Option for the counters | Verdict |
|---|---|
| Keep the binary and only fix the plan case | Rejected. The parallel lane has the other half of the same bug: a delivery there is counted as nothing. One rule, both sides. |
| Count the verdict in every lane | **Chosen.** The verdict already exists for every run; the summary reads it and counts what it says. |
| Leave the parallel lane alone | Rejected. Its summary says "0/2 publishable succeeded" for two deliveries. The same rule, the same fix. |
| Drop the plan counter and report only successes and failures | Rejected. Then a plan is invisible: the summary would say "0/1 succeeded" with no explanation. The plan is a fact about the run and the summary should name it. |

| Option for the summary's shape | Verdict |
|---|---|
| Rename `success` to `delivered` | Rejected. The key is the fleet summary's own contract; renaming it breaks a reader for no gain. The verdict's own word stays in the per-project rows. |
| Keep `success` and add `planned` | **Chosen.** The key keeps its name, its value becomes honest, and the plans get a counter of their own. |
| Add the failure count to the human line | Rejected here. The failure count is in the JSON document and in the error the command returns; the human line names the plans, which is what this change is about. |

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A delivered run | Counted as a success, in every lane. |
| A planned run | Counted as a plan. Never a success, never a failure. |
| A failed run | Counted as a failure. Never a success. |
| A run that failed before a verdict (an unreadable source, a fail-fast skip) | Counted as a failure, as before. |
| `--fail-fast` | Unchanged: only a failure trips it. |
| The exit status | Unchanged: the command still fails when a run failed. |
| The per-project rows | Unchanged. |
| The journal rows | Unchanged. |

## Risks

| Risk | Mitigation |
|---|---|
| A reader counted `fleet.success` as "not failed". | That count was wrong in both directions: it counted a plan as a success and, in the parallel lane, missed a delivery. The new value counts deliveries only, and the plans have their own counter. |
| The human line changes shape. | It gains one clause and keeps every other word. The existing fleet test that reads the summary reads the JSON document, not the line. |
| The fail-fast break moves. | It moves into the failure branch of the same match, so only a failure breaks the loop, exactly as before. |
