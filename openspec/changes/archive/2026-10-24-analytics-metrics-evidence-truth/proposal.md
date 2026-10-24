# Change: analytics-metrics-evidence-truth

## Why

`forge analytics metrics` reports eight measurements that Forge never
made.

Reproduced on the current tree:

```
$ forge analytics metrics /tmp/ana-probe/proj --format json
{
  "metric_id": "quality-healthy",
  "current": 0,
  "note": "",
  "snapshots": [
    {
      "note": "",
      "observed_at": "2026-10-24T06:57:07.364642943+00:00",
      "source": "doctor",
      "value": 0,
      "window": "instant"
    }
  ],
  "state": "present",
  "windows": ["instant"]
}
```

The snapshot says `source: doctor`, `value: 0` and an observation
timestamp. It reads as "the doctor ran at 06:57:07 and found zero
healthy findings". The doctor never ran. Forge did not gather a
single finding, a single agent session, a single queued spec or a
single deployment state, and the report states eight measurements
with a source and a timestamp for each.

The same eight measurements carry the same fabricated zeros in every
surface, because the two callers hand the aggregator a summary they
invented:

```rust
// src/cli/functions_13.rs, before
let doctor = DoctorSummary::default();
let report = aggregate_project_metrics(
    &registry,
    Some(doctor),
    &MetricsAggregateOptions { ... },
)?;
```

```rust
// src/api/admin/assurance/analytics.rs, before
match crate::analytics::aggregate_project_metrics(
    &registry,
    Some(crate::analytics::DoctorSummary::default()),
    &crate::analytics::MetricsAggregateOptions { ... },
)
```

`DoctorSummary::default()` is eight zeros. The aggregator trusted it:

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
```

So the zeros were not an accident of a missing default: they were
manufactured by a default, stamped with a source and a timestamp, and
reported as `present`.

**The canonical contract already forbids this.** The
`external-planes-analytics` capability says it:

> **WHEN** a provider fails or returns an incompatible payload
> **THEN** its metrics are unavailable and the summary does not
> fabricate zeros

The implementation does fabricate zeros, on the path where nothing
failed at all.

**The completion flag then certifies the fabrication.** `complete` is
true when every aggregate is available, and the fabricated metrics are
always available, so a report full of invented zeros is a *complete*
report:

```rust
// src/analytics/projections.rs, before
let complete = aggregates
    .iter()
    .all(|a| a.state != METRIC_STATE_UNAVAILABLE);
```

This is the rule the repair programme is built on: Forge states
something its own gathered evidence does not support. Here the report
is the evidence, and it states measurements that no observation backs.

## What Changes

- **One rule judges every metric**: a metric states a value only when
  Forge observed it. A metric whose source was not gathered is
  `unavailable`, with a note naming the missing evidence, and never a
  fabricated zero.
- **The doctor-sourced metrics follow that rule.** `quality-healthy`,
  `quality-warnings`, `quality-failures`, `agents-running`,
  `specs-queued`, `deployment-running`, `deployment-offline` and
  `deployment-pending` are `unavailable` unless a doctor summary was
  actually gathered.
- **The fabricated summary cannot be built again.** `DoctorSummary`
  loses its `Default` derive, so the all-zero summary the two callers
  passed no longer exists in the type system.
- **The journal detail names what is missing.** The metrics row states
  how many metrics are unavailable, so the row explains itself.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The metric rule | `forge` | Yes | `src/analytics/projections.rs`: the doctor metrics follow the evidence, or are `unavailable`. |
| The doctor summary | `forge` | Yes | `src/analytics/model.rs`: the fabricated default is removed. |
| The metrics CLI | `forge` | Yes | `src/cli/functions_13.rs`: no fabricated summary; the journal detail names the unavailable count. |
| The assurance browser | `forge` | Yes | `src/api/admin/assurance/analytics.rs`: no fabricated summary. |
| The analytics unit tests | `forge` | Yes | `src/analytics/projections_tests.rs`: the rule's unit coverage. |
| The analytics contract test | `forge` | Read only | Its declared R2 wording is updated: the doctor-sourced metrics are `unavailable` unless a summary was gathered. |
| The metrics contract test | `forge` | Added | `tests/analytics_metrics_evidence_truth_contract.rs`. |
| The inspection lane | `forge` | Read only | It never aggregated metrics; change 34 governs it. |
| The `analytics-metrics-evidence-truth` capability | `forge` | Added | this package |
| The `external-planes-analytics` capability | `forge` | Modified | the metrics requirement names the rule. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §33 owns the project-level metrics; the `external-planes-analytics` capability owns the aggregate and its states. |
| Contracts | No field is removed or renamed. The doctor-sourced metrics change their state from `present` to `unavailable` and lose their invented snapshot. `ANALYTICS_CONTRACT_VERSION` is unchanged. |
| Callers | `forge analytics metrics` (human and JSON), its journal row; the assurance browser's `GET .../analytics/metrics`. |
| Persistence | The registry journal detail for a metrics call names the unavailable count. The metrics summary file keeps the same shape. |
| Integrations | None. No adapter is changed; no provider is contacted. |
| Failures | No new error code. The metrics command still exits 0 and still writes its summary and its journal row. |
| Tests | A new contract test `tests/analytics_metrics_evidence_truth_contract.rs`; the analytics unit tests gain the rule's coverage; the existing analytics contract and cross-surface tests stay green. |
| Dependencies | None added. |
| Concerns | Quality: the report stops claiming eight measurements it never made, and the completion flag stops certifying them. Compatibility: a reader of `quality-healthy`'s `current` now reads `null` and the metric's state reads `unavailable`; a reader of `complete` now reads `false`. Security: no new surface; the redaction rule and the credential handling are unchanged. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `analytics-metrics-evidence-truth` | forge | Rust | added | this package |
| `external-planes-analytics` | forge | Rust | modified | the metrics requirement names the rule. |

## Non-goals

- **No new evidence gathering.** This change does not build a doctor
  inventory, an agent-session census or a deployment census. The
  quality counters cannot be gathered without running the doctor,
  which runs the policy adapter and is a per-project probe the
  assurance browser may not perform, so no surface can produce a
  doctor summary today. A later change may gather the local-state
  counters honestly; this change makes their absence visible instead
  of false.
- **The repository-star and growth metrics are unchanged.** They were
  already `unavailable` without an observation, and they stay that
  way.
- **The inspection lane is not in this package.** Change 34 governs it.
- **The provider observation path is unchanged.** A real inspection
  still feeds the repository-star metrics exactly as before.
- **The metrics command's behaviour is unchanged.** It still exits 0,
  still writes its summary file and still writes its journal row.
- **The analytics contract version is unchanged.**
