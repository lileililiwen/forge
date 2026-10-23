# Change: workbench-health-state-truth

## Why

The workbench's health document states the project's health, and its
state is the only place an operator sees it:

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

The `stale` branch is **unreachable**. The doctor's own `healthy` field
is false whenever the report is stale:

```rust
// src/doctor/runner.rs
let healthy = findings
    .iter()
    .all(|f| matches!(f.status, FindingStatus::Pass | FindingStatus::Warn))
    && unmet == 0
    && !stale;
```

So `report.stale` implies `!report.healthy`, the first branch is taken
always, and the health document can never report `stale`. A project
whose only problem is a stale registry observation is reported:

```text
state: issues
findings: manifest-valid pass, profile-known pass, features-compatible pass,
          dependency-drift pass, build-config pass, deployment-config pass,
          repository pass, ci-config pass, docs-present pass,
          driftwatch-config pass, gate-evidence pass,
          registry-observation warn, delivery-state pass,
          maturity-requirements pass, policy-deferred unavailable
```

The state says *issues*. The document it comes with names no failing
finding at all — the one real problem is a `warn`.

**Two surfaces of one report disagree.** The sibling status route reads
the same doctor report. Driven over HTTP on this host, for one project
whose manifest was rewritten after registration:

| Surface | State | What it names |
|---|---|---|
| `GET /v1/admin/projects/{id}` (workbench health) | `issues` | no failing finding; one `warn` |
| `GET /v1/admin/projects/{id}/status` (doctor sub-check) | `stale` | `the registry observation predates the current manifest; re-register to refresh` |

**The canonical capability says the state it cannot produce.** The
`workbench-health-latency` capability requires that "`stale` and
`unavailable` keep their exact meanings", and that the explicit refresh
returns "the full health document (`healthy`/`issues`/`stale`, complete
findings, no deferred finding)". `stale` is one of the three states that
requirement names — and the code could never return it.

**The portal has a badge for a state the API cannot produce.** The
portal's health card maps the states to labels and badges:

```javascript
// frontend/app.js
const HEALTH_LABELS = { healthy: "Healthy", stale: "Stale", issues: "Has issues", deferred: "Deferred", unavailable: "Unavailable" };
const HEALTH_BADGE = { healthy: "state-done", stale: "state-stale", issues: "state-failed", deferred: "state-observed", unavailable: "state-observed" };
```

A `Stale` badge with its own colour, and a rendering branch for it
(`else if (state === "healthy" || state === "stale")` → "No blocking
findings."), are both dead. A stale project wears the red *Has issues*
badge instead. The lifecycle rail counts `healthState === "issues"` as a
bad project, so a stale observation is counted as a real problem there
too.

## What Changes

- **One rule, one place.** The doctor's judgement of its own report
  becomes a typed partition — `Issues`, `Stale`, `Healthy` — that both
  health projections read. The workbench stops re-deriving it from the
  `healthy` field alone.
- **A stale observation reports `stale`.** A report with no blocking
  finding and no unmet control, whose observation is stale, is `stale`
  — not `issues`.
- **The partition is the inverse of the doctor's `healthy` field.** A
  report the doctor does not call healthy is never projected as healthy,
  and the reason it is not healthy is named: a blocking finding is
  `issues`, a stale observation is `stale`.
- **The claims are pinned.** A repository contract check drives the
  real workbench detail and refresh routes and holds them to the
  partition.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The workbench health document | `forge` | Yes | `src/api/workbench.rs`: the state it labels. |
| The doctor's report partition | `forge` | Yes | `src/doctor/model.rs`: the shared partition. |
| The doctor's runner | `forge` | Yes | `src/doctor/runner.rs`: the `healthy` field is expressed through the shared rule. |
| The project-status doctor sub-check | `forge` | Yes | `src/api/status.rs`: the same partition, read instead of re-derived. Its behaviour does not change. |
| The partition's own unit tests | `forge` | Yes | `src/doctor/model.rs`. |
| The workbench health contract test | `forge` | Yes | `tests/workbench_health_state_truth_contract.rs` (new). |
| The `workbench-health-state-truth` capability | `forge` | Added | this package |
| The `workbench-health-latency` capability | `forge` | Modified | The fast-open requirement names the partition. |
| The workbench health card | `forge` | Read only | `frontend/app.js` already renders every state, including `stale`. |
| The lifecycle series rail | `forge` | Read only | It reads the health state; a stale project stops counting as a bad one. |
| The doctor check | `forge` | Read only | Its findings and statuses are unchanged. |
| The release lane | `forge` | Read only | Unchanged. |
| The API contract versions | `forge` | Read only | No field is added, removed or renamed. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §23 (doctor) and §36 (portal) own the health projection; the workbench's own capability owns the fast-open contract. |
| Contracts | The health document's `state` changes for one class of project — a stale observation with no blocking finding — from `issues` to `stale`. No field is added, removed or renamed, and no contract version changes. |
| Callers | The portal's health card, the lifecycle series rail, the workbench detail route, the explicit refresh route. |
| Persistence | Nothing new is persisted. The health document is computed on read. |
| Integrations | None. No new toolchain, network or credential surface. |
| Failures | No new error code. |
| Tests | A new `tests/workbench_health_state_truth_contract.rs` plus unit tests for the partition. |
| Dependencies | None added. |
| Concerns | Quality: the health document states the health the doctor found. Compatibility: the response's shape is unchanged; one classification becomes more precise, and the canonical requirement already demanded it. Security: no new file, process, network or credential surface. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `workbench-health-state-truth` | forge | Rust | added | this package |
| `workbench-health-latency` | forge | Rust | modified | the fast-open requirement names the partition |

## Non-Goals

- **The doctor is unchanged.** Its findings, its statuses, its evidence
  and its `healthy` field keep the same values for every report; only
  the rule behind them becomes readable by the projections.
- **The fast path is unchanged.** A locally-clean project still reports
  `deferred` and still appends the `policy-deferred` finding, and the
  detail GET still runs no external checker.
- **The refresh route is unchanged.** It still runs the live policy
  pass and still returns the full document.
- **A blocking finding is still `issues`.** A failing or unavailable
  finding, or an unmet maturity control, reports `issues` exactly as
  before.
- **The portal is unchanged.** It already renders every state; the
  change only lets the `Stale` badge be reached.
- **The API contract versions are unchanged.**
