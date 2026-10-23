## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The release lane's DriftWatch check | `forge` | Yes. `src/release/engine/prepare.rs`. |
| The doctor's judgement of a policy report | `forge` | Yes. `src/doctor/assessment.rs`. |
| The verdict's own unit tests | `forge` | Yes. `src/doctor/assessment.rs`. |
| The release-check contract test | `forge` | Yes. `tests/release_driftwatch_check_truth_contract.rs` (new). |
| The `release-driftwatch-check-truth` capability | `forge` | Added. |
| The `release-publishing` capability | `forge` | Modified. |
| The `quality-policy-integration` capability | `forge` | Modified. |
| The release lane's doctor check | `forge` | Read only. |
| The release lane's test check | `forge` | Read only. |
| The release lane's Gate check | `forge` | Read only. |
| The release readiness rule | `forge` | Read only. |
| The release apply lane | `forge` | Read only. |
| The policy plane and its adapter | `forge` | Read only. |
| The doctor's policy findings | `forge` | Read only. |

## The Gap

### One rule, two filters

The doctor and the release lane read the same policy report and reach
different verdicts about it, because each re-derives the rule.

The doctor's rollup (`src/doctor/assessment.rs`):

```rust
let status = if fail > 0 {
    FindingStatus::Fail
} else if warn > 0 || stale {
    FindingStatus::Warn
} else if applicable == 0 {
    FindingStatus::Unavailable
} else {
    FindingStatus::Pass
};
```

The release lane's DriftWatch check (`src/release/engine/prepare.rs`):

```rust
if !fail_ids.is_empty() {
    // fail
} else {
    CapturedCheck {
        kind: "driftwatch".to_string(),
        status: CHECK_PASS.to_string(),
        applicable: true,
        evidence: vec!["driftwatch report healthy".to_string()],
        detail: "driftwatch report is healthy".to_string(),
        source_revision: revision.to_string(),
    }
}
```

The release lane's filter is narrower in two ways. It does not count
`WARN`, and it does not count the two conditions that are not severities
at all — a report with no applicable rule, and a report produced against
an older source revision. The doctor's rollup counts all four.

The release lane's own doctor check already counts the doctor's full
blocking set, and already has a `stale` status:

```rust
if report.stale {
    return CapturedCheck {
        kind: "doctor".to_string(),
        status: CHECK_STALE.to_string(),
        /* … */
    };
}
let blocking: Vec<String> = report
    .blocking_findings()
    .iter()
    .map(|f| f.id.clone())
    .collect();
```

So the lane already knows how to say `stale` and how to say `unavailable`.
Only the DriftWatch check cannot.

## The Rule

One typed verdict, owned by the doctor (it is the surface that already
judges a policy report), read by the release lane (it is already a
consumer of the doctor).

```rust
// src/doctor/assessment.rs
/// The doctor's conclusion about one policy report.
pub enum PolicyConclusion {
    /// An applicable rule failed.
    Failing,
    /// The report belongs to an older source state than the one judged.
    Stale,
    /// No rule applied, so nothing was judged.
    Unjudged,
    /// Applicable rules warned; none failed.
    Warning,
    /// Every applicable rule passed.
    Passed,
}

/// The doctor's judgement of one policy report: the conclusion and the
/// ids behind it.
pub struct PolicyVerdict {
    pub conclusion: PolicyConclusion,
    pub failing: Vec<String>,
    pub warning: Vec<String>,
    pub applicable: usize,
    pub stale: bool,
}
```

The conclusion's precedence is the doctor's own, unchanged:

| Order | Condition | Conclusion | The doctor's finding status |
|---|---|---|---|
| 1 | an applicable rule failed | `Failing` | `FAIL` |
| 2 | the report is older than the source state | `Stale` | `WARN` |
| 3 | no rule was applicable | `Unjudged` | `UNAVAILABLE` |
| 4 | an applicable rule warned | `Warning` | `WARN` |
| 5 | otherwise | `Passed` | `PASS` |

The doctor's rollup is expressed through this verdict, so its behaviour
does not change: every row above maps to the status the doctor already
produces for it, including the case where a report is both stale and
unjudged (`Stale` → `WARN`, exactly as the current ordering produces).

The release lane reads the same verdict and maps it to the check
statuses the release contract already defines:

| Conclusion | Check status | Blocks the release? | Evidence |
|---|---|---|---|
| `Failing` | `fail` | yes | the failing rule ids |
| `Stale` | `stale` | yes | the report belongs to an older source revision |
| `Unjudged` | `unavailable` | yes | no rule applied, so there is no policy evidence |
| `Warning` | `pass` | no | the warning rule ids — never the word *healthy* |
| `Passed` | `pass` | no | `driftwatch report healthy` |

`check_readiness` blocks on every status that is not `pass`, so the three
blocking rows block the plan without any change to the readiness rule.

**A warning still does not block.** The canonical release requirement
names three blocking conditions — *fails*, *is unavailable*, *belongs to
an older revision* — and a warning is none of them. The release lane's
own doctor check does not block on a warning finding either. What changes
is the wording: a report that warned is no longer called healthy, and
its evidence names the warnings.

**The word *healthy* is now earned.** It is the evidence of the one case
where every applicable rule passed.

## The Contract Test

`tests/release_driftwatch_check_truth_contract.rs` drives the release
lane with a fake policy adapter — a script pointed at by
`FORGE_DRIFTWATCH_BIN`, the same house pattern as
`tests/workbench_health_latency_contract.rs` — and holds the check to
the five rows above:

- a report whose rule failed → `fail`, and the plan is not ready;
- a report with no applicable rule → `unavailable`, and the plan is not
  ready;
- a report from an older revision → `stale`, and the plan is not ready;
- a report that warned → `pass`, the evidence names the warning, and the
  evidence never says *healthy*;
- a report where every rule passed → `pass` with
  `driftwatch report healthy`.

The test calls the public `forge::release::engine::prepare_release`, so
it reads the same plan the CLI renders — no new public seam and no
private module is opened.

## Failure Boundaries

| Boundary | Behaviour |
|---|---|
| The adapter is missing, times out or emits an unparseable document | The adapter reports `PolicyOutcome::Unavailable` and the check is `unavailable` — unchanged by this change. |
| The report carries a credential in its evidence | The adapter's redaction pipeline still runs before any caller sees the report. The verdict carries ids, not evidence text. |
| The report's contract is unknown | The adapter reports `Unavailable`; unchanged. |
| The plan has no applicable check at all | `check_readiness` reports that no applicable release check exists; unchanged. |
| The project declares no DriftWatch check | The check is `disabled` and never runs; unchanged. |

## Risks

- **A release that used to be ready is now blocked.** For the three
  blocking rows that is the point: the canonical requirement says those
  reports must block. A project whose policy report has no applicable
  rule now needs a policy that applies to it, or a DriftWatch check it
  does not declare.
- **A warninging report still passes.** Deliberate, and stated in the
  capability: a warning is not one of the three blocking conditions.
  What changes is the evidence, which no longer says *healthy*.
- **The doctor's behaviour is unchanged.** The verdict expresses the
  rule the rollup already implemented; the rollup's statuses and wording
  are the same for every report.
