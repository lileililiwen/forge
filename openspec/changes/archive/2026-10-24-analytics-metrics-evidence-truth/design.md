## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The metric rule | `forge` | Yes. `src/analytics/projections.rs`. |
| The doctor summary | `forge` | Yes. `src/analytics/model.rs`. |
| The metrics CLI | `forge` | Yes. `src/cli/functions_13.rs`. |
| The assurance browser | `forge` | Yes. `src/api/admin/assurance/analytics.rs`. |
| The analytics unit tests | `forge` | Yes. `src/analytics/projections_tests.rs`. |
| The metrics contract test | `forge` | Added. `tests/analytics_metrics_evidence_truth_contract.rs`. |
| The `analytics-metrics-evidence-truth` capability | `forge` | Added. |
| The `external-planes-analytics` capability | `forge` | Modified. |
| The analytics contract test | `forge` | Read only (its declared wording). |
| The inspection lane | `forge` | Read only. |
| The analytics contract version | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/analytics/projections.rs, before
let doctor = doctor_summary.unwrap_or_default();
aggregates.push(count_metric(
    METRIC_QUALITY_HEALTHY,
    WINDOW_INSTANT,
    "doctor",
    now_str.clone(),
    doctor.healthy,
));
aggregates.push(count_metric(
    METRIC_QUALITY_WARNINGS,
    WINDOW_INSTANT,
    "doctor",
    now_str.clone(),
    doctor.warnings,
));
```

`count_metric` marks the aggregate `present`, gives it one snapshot
with the source, the value and the observation time, and sets
`current` to the value.

### The summary it invents

```rust
// src/analytics/model.rs, before
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorSummary {
    #[serde(default)]
    pub healthy: i64,
    ...
}
```

`DoctorSummary::default()` is eight zeros, and the two callers pass
it as if it were a reading:

```rust
// src/cli/functions_13.rs, before
let doctor = DoctorSummary::default();
```

```rust
// src/api/admin/assurance/analytics.rs, before
Some(crate::analytics::DoctorSummary::default()),
```

### The completion flag certifies it

```rust
// src/analytics/projections.rs, before
let complete = aggregates
    .iter()
    .all(|a| a.state != METRIC_STATE_UNAVAILABLE);
```

The fabricated metrics are always available, so they never make the
report incomplete.

### The contract already says no

> **WHEN** a provider fails or returns an incompatible payload
> **THEN** its metrics are unavailable and the summary does not
> fabricate zeros

## The Rule

One rule, in the one place the aggregate is built: a metric states a
value only when Forge observed it.

```rust
// src/analytics/projections.rs
/// The doctor-sourced metrics. Their sources are named in the
/// gathered branch below: `doctor` for the quality counters,
/// `agent-sessions` for the agent counter, `spec-registry` for the
/// queued-spec counter and `deploy-state` for the three deployment
/// counters.
const DOCTOR_SOURCED_METRICS: [&str; 8] = [
    METRIC_QUALITY_HEALTHY,
    METRIC_QUALITY_WARNINGS,
    METRIC_QUALITY_FAILURES,
    METRIC_AGENTS_RUNNING,
    METRIC_SPECS_QUEUED,
    METRIC_DEPLOYMENT_RUNNING,
    METRIC_DEPLOYMENT_OFFLINE,
    METRIC_DEPLOYMENT_PENDING,
];

/// The note every doctor-sourced metric carries when no summary was
/// gathered. It names the missing evidence, so the reader knows what
/// was not measured.
const DOCTOR_NOT_GATHERED: &str =
    "no doctor summary was gathered for this run; this metric was not measured";
```

The aggregator branches once, on the summary it was given:

```rust
// src/analytics/projections.rs
match doctor_summary {
    Some(doctor) => {
        aggregates.push(count_metric(
            METRIC_QUALITY_HEALTHY,
            WINDOW_INSTANT,
            "doctor",
            now_str.clone(),
            doctor.healthy,
        ));
        // ... the other seven, exactly as before
    }
    None => {
        for metric_id in DOCTOR_SOURCED_METRICS {
            aggregates.push(unavailable_metric(metric_id, DOCTOR_NOT_GATHERED));
        }
    }
}
```

`unavailable_metric` is the same shape the repository-star metrics
already use when no provider answered:

```rust
/// A metric nobody measured. It carries no snapshot, no value and no
/// window: an unmeasured metric has no reading to report.
fn unavailable_metric(metric_id: &str, note: &str) -> MetricAggregate {
    MetricAggregate {
        metric_id: metric_id.to_string(),
        state: METRIC_STATE_UNAVAILABLE.to_string(),
        snapshots: Vec::new(),
        current: None,
        windows: Vec::new(),
        note: note.to_string(),
    }
}
```

### The fabricated summary cannot be built again

```rust
// src/analytics/model.rs, before
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorSummary { ... }
```

The `Default` derive goes. `DoctorSummary::default()` no longer
exists, so a caller cannot hand the aggregator an eight-zero summary
that reads as a reading. A summary that is really gathered is built
field by field, from a real observation.

### What each surface does

| Surface | Before | After |
|---|---|---|
| the CLI's doctor summary | `DoctorSummary::default()` (eight zeros) | none gathered; the metrics are `unavailable` |
| the assurance browser's doctor summary | `DoctorSummary::default()` (eight zeros) | none gathered; the metrics are `unavailable` |
| the doctor-sourced metrics | `present`, `current: 0`, one invented snapshot | `unavailable`, `current: null`, no snapshot |
| the report's `complete` | `true` with eight invented readings | `false` while a metric is unmeasured |
| the metrics journal detail | `complete={} aggregates={}` | `complete={} aggregates={} unavailable={}` |
| the registry-count metric | `present` with the real count | unchanged |
| the repository-star metrics | `unavailable` without an observation | unchanged |
| a caller that gathers a real summary | real counts | unchanged |

### Why the completion flag becomes false

`complete` means every metric in the report was measured. When the
doctor-sourced metrics are unmeasured, the report is not complete, and
saying otherwise would certify eight invented readings. The metrics
command's journal row already distinguishes the two: it writes
`done` for a complete report and `partial` for an incomplete one.

### Why the summary file keeps its shape

The persisted metrics summary keeps every metric id, so a reader can
see which metrics were unmeasured. Only the state and the value of
the doctor-sourced metrics change, from an invented reading to an
explicit gap.

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A metrics run with no doctor summary | the eight doctor-sourced metrics are `unavailable` with the note; the registry count is still `present`; the report is not complete; the journal row is `partial`. |
| A metrics run with a gathered doctor summary | the eight metrics carry the gathered counts, exactly as before; the report is complete when the repository metrics are available too. |
| A metrics run with external observations | the repository-star and growth metrics carry the observed values, exactly as before. |
| The assurance browser's metrics route | the same as the CLI: no doctor summary is gathered, so the doctor-sourced metrics are `unavailable`. The route still probes nothing and writes nothing. |
| `forge analytics metrics --all` | the same rule over the whole registry; the synthetic project id and the summary path are unchanged. |
| The metrics command's exit code | unchanged: 0. |
| The metrics summary file | unchanged in shape; the doctor-sourced metrics carry the explicit gap. |
| The inspection lane | unchanged; change 34 governs it. |

## Risks

| Risk | Mitigation |
|---|---|
| The report loses eight counters. | It never had them: the counters were zeros Forge invented. A reader now sees an explicit gap instead of a number that means nothing. The doctor findings are not stored anywhere, so no surface can gather them without running the doctor, which the assurance browser may not do. |
| A reader of `quality-healthy`'s `current` breaks. | The value was 0 because nothing was measured. `current` is `null` and the state is `unavailable`, which is the state the contract already defines for a metric nobody answered. |
| A reader of `complete` breaks. | `complete` now means what it says: every metric was measured. The journal row distinguishes `done` from `partial`. |
| The `Default` derive is needed elsewhere. | It is not: the only two uses were the two fabricated summaries this change removes. |
| A future caller re-introduces the zeros. | `DoctorSummary::default()` no longer compiles. |
