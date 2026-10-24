# Change: analytics-inspect-verdict-truth

## Why

`forge analytics inspect --dry-run` reports every provider as
`available`, and records a completed inspection in the registry
journal.

Reproduced on the current tree:

```
$ forge analytics inspect /tmp/ana-probe/proj --dry-run
analytics contract=0.1.0 project=ana-probe enabled=true default_window=7d
  plane=content provider=unified-content project_ref=slug status=available source=dry-run observed_at=2026-10-24T05:47:30Z note=dry-run: adapter was not invoked
    evidence: would invoke adapter for project_ref=`slug`
  plane=repository provider=github-analytics project_ref=owner/repo status=available source=dry-run observed_at=2026-10-24T05:47:30Z note=dry-run: adapter was not invoked
    evidence: would invoke adapter for project_ref=`owner/repo`
```

and the append-only row the same run wrote:

```
('analytics', 'ana-probe', 'done', 'inspect: project=ana-probe enabled=true observations=2 healthy=true dry_run=true')
```

The observation says `status=available` and the journal says
`done, healthy=true` for a run that invoked no adapter. The
observation's own note says the opposite in the same breath:
`dry-run: adapter was not invoked`. One document, two answers, and
the evidence a later reader consults claims a health check that never
happened.

**The status is hard-coded at the point where nothing was checked.**

```rust
// src/analytics/projections.rs, before
if options.dry_run {
    return Ok(HealthObservation {
        ...
        status: STATUS_AVAILABLE.to_string(),
        source: "dry-run".to_string(),
        ...
        note: "dry-run: adapter was not invoked".to_string(),
    });
}
```

The status vocabulary already has a word for "this was never
checked" — it is called `unavailable` for a provider that could not
be reached — but the dry run had no way to say "planned", so it said
`available`.

**The report's own summary contradicts its observations.** The
`available` field is documented as "`true` when at least one provider
is `available`", but the enabled path writes it as a constant:

```rust
// src/analytics/projections.rs, before
let report = ExternalPlaneReport {
    ...
    available: false,
};
```

So an inspection whose observation says `available` still reports
`"available": false`, and the disabled path writes `"available":
true` while every observation says `disabled`. The field states
something the report's own evidence does not support, in both
directions.

**The health verdict has the same shape.** `healthy()` accepts
`available`, `disabled`, `unconfigured` and `ambiguous-mapping`, and
the dry run hands it `available`:

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

and the CLI writes the journal row from that boolean:

```rust
// src/cli/analytics.rs, before
let state = if report.healthy() { "done" } else { "rejected" };
```

A boolean cannot say "planned", so a plan is recorded as a completed
inspection. This is the rule the repair programme is built on: Forge
states something its own gathered evidence does not support. A dry
run's whole value is that it is safe, and its journal row is the
evidence that it was.

## What Changes

- **One rule judges an inspection**, in the one place the report is
  built: a typed verdict that says what the inspection did —
  `delivered`, `planned` or `failed` — read from the observations the
  inspection actually made.
- **A dry run's observations are `planned`,** never `available`. The
  status names what Forge knows: nothing was checked.
- **`healthy` is true only for `delivered`.** A plan checked nothing.
- **The report's `available` field is derived from its
  observations,** so it can no longer contradict them.
- **The journal row follows the one verdict.** A plan is recorded as
  `planned`, never as `done`, and the detail names the verdict.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The verdict rule | `forge` | Yes | `src/analytics/model.rs`: `AnalyticsVerdict`, `verdict_for_observations`, `ExternalPlaneReport::from_observations`. |
| The observation statuses | `forge` | Yes | `src/analytics/constants.rs`: the `planned` status. |
| The inspection | `forge` | Yes | `src/analytics/projections.rs`: the dry-run status, the report construction. |
| The analytics CLI | `forge` | Yes | `src/cli/analytics.rs`: the journal state and detail follow the verdict. |
| The analytics module contract | `forge` | Yes | `src/analytics/mod.rs`: the persistence paragraph names the journal states. |
| The metrics lane | `forge` | Read only | It reads observations from a real inspection, never from a dry run. |
| The assurance browser | `forge` | Read only | It never calls `inspect_external_planes`; it only reads the metrics aggregate. |
| The MCP surface | `forge` | Read only | It does not expose the analytics inspection. |
| The `analytics-inspect-verdict-truth` capability | `forge` | Added | this package |
| The `external-planes-analytics` capability | `forge` | Modified | the inspection requirement names the verdict. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §32 and §33 own the external content and analytics plane; the `external-planes-analytics` capability owns the health observations and the inspection report. |
| Contracts | The observation vocabulary gains one status, `planned`. The report gains one field, `verdict`. No field is removed or renamed, and `ANALYTICS_CONTRACT_VERSION` is unchanged. |
| Callers | `forge analytics inspect` (human and JSON), its journal row; the metrics lane, which reads a real inspection's observations. |
| Persistence | The registry journal state for a planned inspection becomes `planned`, and for a failed provider becomes `failed`. |
| Integrations | None. No adapter is changed; a dry run still invokes no adapter. |
| Failures | No new error code. An inspection still exits 0 in every case; the journal row carries the verdict. |
| Tests | A new contract test `tests/analytics_inspect_verdict_truth_contract.rs`; the analytics unit test that enshrined the old dry-run status is rewritten; the existing analytics contract and cross-surface tests stay green. |
| Dependencies | None added. |
| Concerns | Quality: the journal stops claiming a health check that never happened, and the report's summary agrees with its observations. Compatibility: a reader matching the observation status `available` on a dry run now sees `planned` and reads the verdict; a reader matching the journal state `done` for a dry run now sees `planned`; the report's `available` field is true for a healthy inspection where it was hard-coded false. Security: no new surface; no secret is involved, and the evidence redaction rule is unchanged. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `analytics-inspect-verdict-truth` | forge | Rust | added | this package |
| `external-planes-analytics` | forge | Rust | modified | the inspection requirement names the verdict. |

## Non-goals

- **No adapter behaviour changes.** A dry run still invokes no adapter
  and a real inspection still invokes the adapter with argument arrays
  and a bounded timeout.
- **The evidence and the notes are unchanged.** The dry run still
  reports which provider it would contact and with which project
  reference.
- **The metrics lane is not in this package.** `forge analytics
  metrics` aggregates observations from a real inspection and has its
  own completeness rule; that is separate work.
- **The assurance browser is not in this package.** It never calls the
  inspection.
- **A disabled analytics block keeps its verdict.** A block that is
  opted out has nothing to plan: the inspection delivered the
  manifest's evidence that the provider is disabled, and the
  observations still say `disabled`.
- **The analytics contract version is unchanged.** The report gains a
  field; nothing is removed.
