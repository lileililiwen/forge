## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The verdict rule | `forge` | Yes. `src/analytics/model.rs`. |
| The observation statuses | `forge` | Yes. `src/analytics/constants.rs`. |
| The inspection | `forge` | Yes. `src/analytics/projections.rs`. |
| The analytics CLI | `forge` | Yes. `src/cli/analytics.rs`. |
| The analytics module contract | `forge` | Yes. `src/analytics/mod.rs`. |
| The analytics unit tests | `forge` | Yes. `src/analytics/projections_tests.rs`. |
| The analytics contract test | `forge` | Added. `tests/analytics_inspect_verdict_truth_contract.rs`. |
| The `analytics-inspect-verdict-truth` capability | `forge` | Added. |
| The `external-planes-analytics` capability | `forge` | Modified. |
| The metrics lane | `forge` | Read only. |
| The assurance browser | `forge` | Read only. |
| The MCP surface | `forge` | Read only. |
| The analytics contract version | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/analytics/projections.rs, before
if options.dry_run {
    return Ok(HealthObservation {
        provider: provider_label(entry.provider).to_string(),
        plane: plane.to_string(),
        project_ref: entry.project_ref.clone(),
        status: STATUS_AVAILABLE.to_string(),
        observed_at: now.to_rfc3339(),
        source: "dry-run".to_string(),
        evidence: vec![format!(
            "would invoke adapter for project_ref=`{}`",
            entry.project_ref
        )],
        note: "dry-run: adapter was not invoked".to_string(),
    });
}
```

The status says `available` in the same struct that says
`dry-run: adapter was not invoked`.

### The same run says the opposite

The journal row the CLI wrote for that run:

```
('analytics', 'ana-probe', 'done', 'inspect: project=ana-probe enabled=true observations=2 healthy=true dry_run=true')
```

### The report's summary contradicts its observations

```rust
// src/analytics/projections.rs, before
let report = ExternalPlaneReport {
    contract: ANALYTICS_CONTRACT_VERSION.to_string(),
    project_id: project_id.to_string(),
    enabled: true,
    default_window_days: config.default_window_days,
    observations,
    available: false,
};
```

`available` is a constant in the enabled path and the opposite
constant in the disabled path:

```rust
// src/analytics/projections.rs, before
return Ok(ExternalPlaneReport {
    ...
    available: true,
});
```

The field is documented as "`true` when at least one provider is
`available`", so in the enabled path it contradicts an observation
that says `available`, and in the disabled path it is true while every
observation says `disabled`.

### The health verdict cannot say "planned"

```rust
// src/analytics/model.rs, before
pub fn healthy(&self) -> bool {
    for obs in &self.observations {
        match obs.status.as_str() {
            STATUS_AVAILABLE | STATUS_DISABLED | STATUS_UNCONFIGURED | STATUS_AMBIGUOUS => continue,
            _ => return false,
        }
    }
    true
}
```

and the writer that turns it into the journal state:

```rust
// src/cli/analytics.rs, before
let state = if report.healthy() { "done" } else { "rejected" };
```

A boolean has two values, so a plan has to be `done` or `rejected`.
Neither is true: the run inspected nothing.

## The Rule

One rule, in the one place the report is built:

```rust
// src/analytics/model.rs
/// The inspection's verdict.
///
/// The verdict is what the inspection did. An inspection that
/// invoked no adapter planned; an inspection whose providers were
/// all healthy delivered; an inspection whose provider failed is
/// failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnalyticsVerdict {
    /// Every provider was inspected and none failed.
    Delivered,
    /// No adapter was invoked. The report is a plan.
    Planned,
    /// A provider was inspected and did not report a healthy state.
    Failed,
}

impl AnalyticsVerdict {
    pub fn label(self) -> &'static str {
        match self {
            AnalyticsVerdict::Delivered => "delivered",
            AnalyticsVerdict::Planned => "planned",
            AnalyticsVerdict::Failed => "failed",
        }
    }

    /// The registry journal state for this verdict.
    pub fn journal_state(self) -> &'static str {
        match self {
            AnalyticsVerdict::Delivered => "done",
            AnalyticsVerdict::Planned => "planned",
            AnalyticsVerdict::Failed => "failed",
        }
    }
}

/// The one rule that judges an inspection.
///
/// The verdict is read from the observations the inspection actually
/// made: a `planned` observation was never checked, so the run
/// planned; an observation that is not healthy fails the run.
pub fn verdict_for_observations(observations: &[HealthObservation]) -> AnalyticsVerdict {
    if observations.iter().any(|obs| obs.status == STATUS_PLANNED) {
        return AnalyticsVerdict::Planned;
    }
    if observations
        .iter()
        .any(|obs| !HEALTHY_STATUSES.contains(&obs.status.as_str()))
    {
        return AnalyticsVerdict::Failed;
    }
    AnalyticsVerdict::Delivered
}
```

The report is built through one constructor, so no caller can hand
out a report whose summary contradicts its evidence:

```rust
impl ExternalPlaneReport {
    /// Build the report from the observations the inspection made.
    /// The verdict and the availability are read from them.
    pub fn from_observations(
        contract: String,
        project_id: String,
        enabled: bool,
        default_window_days: u32,
        observations: Vec<HealthObservation>,
    ) -> Self {
        Self {
            contract,
            project_id,
            enabled,
            default_window_days,
            available: available_for_observations(&observations),
            verdict: verdict_for_observations(&observations),
            observations,
        }
    }

    /// `true` when the inspection delivered healthy evidence.
    pub fn healthy(&self) -> bool {
        self.verdict == AnalyticsVerdict::Delivered
    }
}
```

### The statuses the rule reads

| Status | Meaning | Healthy evidence |
|---|---|---|
| `available` | the adapter ran and reported a healthy provider | yes |
| `disabled` | the manifest opts the provider out | yes |
| `unconfigured` | no entry was declared | yes |
| `ambiguous-mapping` | the provider refused the mapping | yes |
| `planned` | the adapter was not invoked | no — it was never checked |
| `unavailable` | the adapter could not run, failed, or is not supported | no |

### What each surface does with the verdict

| Surface | Before | After |
|---|---|---|
| the dry run's observation status | `available` | `planned` |
| the report's `available` field | a constant per path | derived from the observations |
| the report's `healthy` | "every status is healthy" | true only for `delivered` |
| the report's new `verdict` field | absent | `delivered`, `planned` or `failed` |
| the CLI's journal row | `done` for a dry run | `planned` for a dry run |
| the CLI's journal detail | `healthy={}` | `verdict={} healthy={}` |
| the CLI's exit code | 0 | 0 (unchanged) |

### Why a plan is not a failure

The inspection command reports what it found; a provider that is not
healthy is a reportable outcome, not a command failure, so the exit
code stays 0 in every case. The verdict separates the three outcomes
in the report and in the journal, which is where a later reader looks.

### Why the journal state for a failed provider changes

Today a provider that failed writes the state `rejected`, which the
product uses for a request the surface refused — for example a
project with no `analytics:` block at all, which is still `rejected`
after this change. A provider that was inspected and is not healthy is
a failure, so the state becomes `failed`, the word the release,
publish and upgrade lanes already use for the same thing.

### Why a disabled block keeps its verdict

A block whose master switch is `false` has nothing to plan: the
inspection delivered the manifest's evidence that the provider is
opted out, no adapter was needed, and the observations still say
`disabled`. The verdict is `delivered` and the report is healthy,
exactly as today. The dry run of a disabled block is the same run.

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A dry run of an enabled block | the observations are `planned`; the verdict is `planned`; the report is not healthy; the journal records `planned`. |
| A real inspection whose adapter reports a healthy provider | the observation is `available`; the verdict is `delivered`; the report is healthy; the journal records `done`. |
| A real inspection whose adapter cannot run, fails, or is not supported | the observation is `unavailable`; the verdict is `failed`; the report is not healthy; the journal records `failed`. |
| A disabled `analytics:` block | the observations are `disabled`; the verdict is `delivered`; the report is healthy; the journal records `done`. |
| A per-entry `enabled: false` provider | the observation is `disabled`; the verdict follows the other observations. |
| An enabled block with no provider entry | there is no observation to judge; the verdict is `delivered`. |
| A project with no `analytics:` block | unchanged: the surface refuses the request with `error[analytics-invalid]` and journals `rejected`. |
| The metrics lane | unchanged: it aggregates the observations of a real inspection, which are never `planned`. |
| The assurance browser | unchanged: it never calls the inspection. |
| The MCP surface | unchanged: it does not expose the inspection. |
| The adapter invocation | unchanged: argument arrays, a bounded timeout, no shell. |
| The evidence redaction | unchanged: the observation evidence and notes still go through `redact_credentials`. |

## Risks

| Risk | Mitigation |
|---|---|
| A reader of the observation status `available` on a dry run breaks. | The status was a false claim: the adapter was not invoked. The verdict and the observation's `source` (`dry-run`) and note say what happened. |
| A reader of the journal state `done` for a dry run breaks. | The state becomes `planned` and the detail names the verdict and the dry run, so the row stays unambiguous. |
| A reader of the report's `available` field breaks. | The field was a constant that contradicted the observations. It is now `true` for a healthy inspection and `false` for a dry run, a failed provider and a disabled block that has at least one enabled provider. |
| The metrics lane loses its repository observations. | The metrics lane inspects with `dry_run: false`, so it never sees a `planned` observation. |
| The analytics unit test that asserted the old dry-run status fails. | That test enshrined the false claim. It is rewritten to assert the planned status and the planned verdict. |
| The contract version changes shape. | The report gains one field and the observation vocabulary gains one status. Nothing is removed or renamed, and `ANALYTICS_CONTRACT_VERSION` stays `0.1.0`. |
