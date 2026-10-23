## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The workbench health document | `forge` | Yes. `src/api/workbench.rs`. |
| The doctor's report partition | `forge` | Yes. `src/doctor/model.rs`. |
| The doctor's runner | `forge` | Yes. `src/doctor/runner.rs`. |
| The project-status doctor sub-check | `forge` | Yes. `src/api/status.rs` — the same partition, read instead of re-derived. |
| The partition's own unit tests | `forge` | Yes. `src/doctor/model.rs`. |
| The workbench health contract test | `forge` | Yes. `tests/workbench_health_state_truth_contract.rs` (new). |
| The `workbench-health-state-truth` capability | `forge` | Added. |
| The `workbench-health-latency` capability | `forge` | Modified. |
| The workbench health card | `forge` | Read only. `frontend/app.js`. |
| The lifecycle series rail | `forge` | Read only. |
| The doctor check | `forge` | Read only. |
| The release lane | `forge` | Read only. |

## The Gap

### The state is derived from a field that cannot separate the two reasons

The doctor's `healthy` field is one boolean over two different facts: a
report can be unhealthy because it carries blocking evidence, or
because its observation is stale. The workbench asks only the boolean
and so cannot tell the two apart:

```rust
// src/api/workbench.rs
let label = if !report.healthy {
    "issues"
} else if report.stale {
    "stale"
} else if policy_ran {
    "healthy"
} else {
    "deferred"
};
```

```rust
// src/doctor/runner.rs
let healthy = findings
    .iter()
    .all(|f| matches!(f.status, FindingStatus::Pass | FindingStatus::Warn))
    && unmet == 0
    && !stale;
```

`!report.healthy` is true when the report is stale, so the first branch
takes the stale report before the second branch is ever reached. The
second branch is dead code, and with it the `stale` state.

### The sibling projection already knows the difference

The project-status route reads the same report and already separates the
two, because it asks the doctor for its blocking set rather than for
the boolean:

```rust
// src/api/status.rs
let blocking = report.blocking_findings();
let unmet = report.unmet_controls().len();
if !blocking.is_empty() || unmet > 0 {
    // issues
} else if report.stale {
    // stale
} else {
    // healthy
}
```

That is the partition. The workbench cannot express it because the
partition is not a value the doctor hands out — it is written twice, in
two projections, in two files.

## The Rule

One typed partition, owned by the doctor (it is the surface that judges
the report), read by both projections.

```rust
// src/doctor/model.rs
/// How the doctor judges its own report.
pub enum HealthPartition {
    /// The report carries blocking evidence: a failing or unavailable
    /// finding, or an unmet maturity control.
    Issues,
    /// The report carries no blocking evidence and its observation is
    /// stale.
    Stale,
    /// The report carries no blocking evidence and its observation is
    /// current.
    Healthy,
}

impl DoctorReport {
    /// The report's own health partition, judged by the doctor's
    /// blocking set.
    pub fn health_partition(&self) -> HealthPartition {
        if !self.blocking_findings().is_empty() || !self.unmet_controls().is_empty() {
            HealthPartition::Issues
        } else if self.stale {
            HealthPartition::Stale
        } else {
            HealthPartition::Healthy
        }
    }
}
```

The partition is the inverse of the doctor's `healthy` field, and the
runner's `healthy` field is now expressed through it, so the two cannot
drift:

```rust
// src/doctor/runner.rs
let mut report = DoctorReport { /* … */ healthy: false, stale };
report.healthy = matches!(report.health_partition(), HealthPartition::Healthy);
```

`Healthy` ⟺ no blocking finding, no unmet control, not stale. That is
exactly the runner's current expression — `all(f => Pass | Warn)` is
"no `Fail` and no `Unavailable`" — so the field keeps the same value for
every report.

The two projections then map the partition to their own vocabularies:

| Partition | The workbench health document | The project-status doctor sub-check |
|---|---|---|
| `Issues` | `issues` | `issues` |
| `Stale` | `stale` | `stale` |
| `Healthy` | `healthy` when the live policy pass ran, `deferred` when it did not | `healthy` |

The workbench's `deferred` and `unavailable` states are its own: the
first says the live policy check did not run for this view, the second
that the project directory could not be read. Neither is a partition of
the doctor's report.

## The Contract Test

`tests/workbench_health_state_truth_contract.rs` drives the two real
routes — the workbench detail GET and the explicit refresh — through the
public `forge::api::handle`, and holds them to the partition:

- a project whose only problem is a stale observation → the detail
  reports `stale`, and the refresh reports `stale`;
- a project with a failing local check → `issues` on both routes;
- a locally-clean project → the detail reports `deferred` with the
  `policy-deferred` finding, and the refresh reports `healthy`;
- an unreadable project directory → `unavailable`.

The test also drives the project-status route for the same projects and
asserts that its doctor sub-check carries the same partition, so the
two projections are pinned to one rule.

The stale fixture is a registered project whose `forge.yaml` is written
after the registration, which is what makes the registry observation
stale — the same way `tests/project_status_evidence_truth_contract.rs`
builds its stale case.

## Failure Boundaries

| Boundary | Behaviour |
|---|---|
| The project directory is unreadable | `unavailable_health()` — unchanged. |
| The live policy check did not run for the view (the fast path) | The document appends the `policy-deferred` finding and a locally-clean project reports `deferred` — unchanged. |
| The doctor report could not be produced | `unavailable` with a safe reason — unchanged. |
| The report carries a failing or unavailable finding | `issues` — unchanged. |
| The report carries an unmet maturity control | `issues` — unchanged. |
| The report is stale and carries nothing else | `stale` — the change. |

## Risks

- **A stale project stops counting as a bad project.** The portal's
  lifecycle rail counts `healthState === "issues"` as bad, and the
  health card's badge changes from the red *Has issues* to the
  *Stale* badge. That is the point: the canonical requirement says
  `stale` keeps its exact meaning, and the badge for it already
  exists.
- **A stale project on the fast path still carries the
  `policy-deferred` finding.** The fast path did not run the policy
  check, so the document still says so; the state now names the stale
  observation instead of hiding it behind *issues*.
- **The doctor's `healthy` field does not change.** The partition is
  expressed through the same facts the runner already used.
- **The project-status route does not change.** It already reported
  `stale`; it now reads the partition instead of re-deriving it.
