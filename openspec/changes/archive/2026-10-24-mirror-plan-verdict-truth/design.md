## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The verdict rule | `forge` | Yes. `src/distribution/model.rs`. |
| The mirror plan and apply | `forge` | Yes. `src/distribution/engine.rs`. |
| The human renderer | `forge` | Yes. `src/distribution/engine.rs`. |
| The mirror CLI | `forge` | Yes. `src/cli/gitops.rs`. |
| The mirror contract test | `forge` | Added. `tests/mirror_plan_verdict_truth_contract.rs`. |
| The `mirror-plan-verdict-truth` capability | `forge` | Added. |
| The `repository-distribution` capability | `forge` | Modified. |
| The mirror report's fields | `forge` | Read only, except the added `verdict`. |
| The MCP mirror method | `forge` | Read only. |
| The release mirror stage | `forge` | Read only. |
| The distribution state file | `forge` | Read only. |
| The distribution contract version | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/distribution/engine.rs, before
let report = MirrorReport {
    contract: DISTRIBUTION_CONTRACT_VERSION.to_string(),
    project_id: request.project_id.clone(),
    primary,
    refs: request.refs.clone(),
    dry_run: true,
    retry_failed: request.retry_failed,
    state_path: state_file.display().to_string(),
    outcomes,
    note: "plan only; no push was attempted".to_string(),
    healthy: true,
};
```

The note and the `healthy` field are in the same struct and they
disagree.

### The same configuration, two verdicts

A mirror declared on a provider that is planned but not supported
(`gitlab`, `codeberg`) produces an `unavailable` outcome in both
runs:

```rust
// src/distribution/engine.rs, before, plan_mirror
let status = match mirror_support_status(entry.provider) {
    MirrorSupportStatus::Planned => "unavailable",
    _ => "would-push",
};
```

```rust
// src/distribution/engine.rs, before, apply_mirror
if mirror_support_status(entry.provider) == MirrorSupportStatus::Planned {
    for ref_name in &request.refs {
        outcomes.push(MirrorRemoteOutcome {
            status: "unavailable".to_string(),
            note: format!("provider `{}` is planned for a later change; no write attempted ..."),
```

The apply reads `unavailable` as a failure and reports `healthy:
false`. The plan reports `healthy: true` for the same outcome.

### The journal

```rust
// src/cli/gitops.rs, before
let detail = format!(
    "mirror `{}` refs={} primary={} healthy={} dry_run={} retry={}",
    ...
);
let state_label = if report.healthy() { "done" } else { "partial" };
```

The append-only evidence records a completed mirror operation, with
`healthy=true dry_run=true` in its detail, for a run that pushed
nothing.

### The two mirror surfaces disagree

After the release dry-run change, the release lane's mirror stage
reports a plan as `disabled`. The standalone mirror command reports
the same plan as healthy and journals it as `done`. The same plan,
two verdicts, on two surfaces.

## The Rule

One rule, in the one place the report is built:

```rust
// src/distribution/model.rs
/// The mirror run's verdict.
///
/// The verdict is what the run did. A plan is not a delivery, and a
/// delivery that did not reach every enabled remote is not a
/// delivery either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MirrorVerdict {
    /// Every enabled remote reached the requested ref.
    Delivered,
    /// A write was attempted and at least one enabled remote did
    /// not reach the requested ref.
    Partial,
    /// No write was attempted. The report is a plan.
    Planned,
}

/// The one rule that judges a mirror run.
///
/// A run that attempted no write is a plan, whatever its per-remote
/// outcomes say. A run that attempted a write is a delivery only
/// when every enabled remote reached the requested ref.
pub fn verdict_for_outcomes(
    dry_run: bool,
    outcomes: &[MirrorRemoteOutcome],
) -> MirrorVerdict {
    if dry_run {
        return MirrorVerdict::Planned;
    }
    if outcomes.is_empty() {
        return MirrorVerdict::Partial;
    }
    let complete = outcomes
        .iter()
        .all(|o| matches!(o.status.as_str(), "delivered" | "skipped" | "disabled"));
    if complete {
        MirrorVerdict::Delivered
    } else {
        MirrorVerdict::Partial
    }
}
```

`MirrorReport` gains the verdict, and `healthy` is derived from it:

| The report | `verdict` | `healthy` |
|---|---|---|
| every enabled remote reached the ref | `delivered` | `true` |
| a write was attempted and a remote did not reach the ref | `partial` | `false` |
| no write was attempted | `planned` | `false` |

### What each surface does with the verdict

| Surface | Before | After |
|---|---|---|
| `plan_mirror` | `healthy: true`, hard-coded | `verdict: Planned`, `healthy: false` |
| `apply_mirror` | `healthy_check()` | `verdict_for_outcomes(false, &outcomes)` |
| `render_report_human` | no verdict line | `verdict: {label}` |
| `cmd_mirror` journal state | `done` for a plan | `planned` for a plan |
| `cmd_mirror` journal detail | `healthy={}` | `verdict={} healthy={}` |
| `cmd_mirror` exit code | `healthy` decides | `verdict` decides: `delivered` and `planned` exit 0, `partial` exits 1 |

### Why a plan is not a failure

The command's exit code is not `healthy` any more; it is the verdict.
A plan that ran to completion is a success of the plan, and it exits
0 exactly as it does today. A partial delivery still exits 1 with the
per-remote evidence on stdout, exactly as it does today.

### Why the plan and the apply no longer contradict each other

They do not contradict each other after this change: the plan's
verdict is `planned` and the apply's verdict is `partial`, and each
one says what that run did. The `unavailable` outcome appears in both
reports, so the reader sees the same fact about the provider in both.

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A plan of any configuration | `planned`; the journal records `planned`; the exit code is 0. |
| An apply where every enabled remote reached the ref | `delivered`; `healthy` is true; the journal records `done`; the exit code is 0. |
| An apply where a remote failed, diverged or is unavailable | `partial`; `healthy` is false; the journal records `partial`; the exit code is 1 with the per-remote evidence. |
| An apply with no remote at all | `partial`; nothing was delivered. |
| A retry that skips an already delivered ref | `delivered`; a skipped ref is a delivered ref. |
| A mirror that is disabled in the manifest | `delivered` or `partial` by the other remotes; the disabled remote's outcome stays `disabled` and no write is attempted to it. |
| The MCP `mirror_project` method | Unchanged: it returns the report, and the report now carries the verdict. |

## Risks

| Risk | Mitigation |
|---|---|
| A dry run starts exiting 1. | The exit code follows the verdict, not `healthy`; a contract test asserts a dry run exits 0. |
| A reader of `healthy` on a dry run breaks. | `healthy` already meant "the refs reached their remotes"; a plan never reached a remote. The verdict is added alongside it, and the renderer and the JSON document both name it. |
| A reader of the journal state `done` for a dry run breaks. | The journal detail names the verdict and the dry run, so the row is still unambiguous; the state becomes `planned`. |
| The per-remote evidence changes. | The rule only judges the run; the outcomes, their notes, their evidence and their recovery lines are untouched, and the existing distribution contract tests stay green. |
| The release lane's mirror stage changes. | It does not read the mirror report's health; it reports a plan as `disabled` and keeps the plan's evidence. |
| The state file changes. | The verdict is derived from the outcomes in memory; the state file is written exactly as before. |
