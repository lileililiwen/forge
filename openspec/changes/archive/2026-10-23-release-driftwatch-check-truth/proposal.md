# Change: release-driftwatch-check-truth

## Why

`forge release prepare` captures one check per configured kind and judges
the plan `ready` when every applicable check passed. The DriftWatch check
judges a policy report with a filter of its own:

```rust
// src/release/engine/prepare.rs
for finding in &report.findings {
    if !finding.applicable {
        continue;
    }
    if matches!(finding.severity, crate::policy::PolicySeverity::Fail) {
        fail_ids.push(finding.id.clone());
    }
}
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

Only `FAIL` is counted. Everything else — a warning, a report with no
applicable rule at all, a report produced against an older source
revision — becomes `pass` with the literal words *"driftwatch report
healthy"*.

The doctor judges the **same report** with the **same rule's** full
severity set, in `src/doctor/assessment.rs`:

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

Three reports, two verdicts. Each row below was produced on this host
with a fake DriftWatch adapter (`FORGE_DRIFTWATCH_BIN`) and one project
that declares `release.checks: [driftwatch]`.

| The policy report says | `forge doctor` says | `forge release prepare` says |
|---|---|---|
| one rule warned | `[WARN] driftwatch-policy` — `1 finding(s): 0 pass, 1 warn, 0 fail` | `driftwatch pass … driftwatch report is healthy` |
| no rule applied to this profile | `[UNAVAILABLE] driftwatch-policy` — `1 finding(s): 0 pass, 0 warn, 0 fail` | `driftwatch pass … driftwatch report is healthy` |
| the report was produced against an older revision | `[WARN] driftwatch-policy` — `source revision older than current forge.yaml` | `driftwatch pass … driftwatch report is healthy` |

**A report the doctor calls unavailable passes the release.** The
canonical policy capability says the opposite in its own Purpose:
*"unavailable (never pass)"*, and its boundary scenario says *"the reason
is preserved rather than manufacturing a detector result"*. A policy that
does not apply to a profile is exactly the case the doctor refuses to
call a pass — and the release lane calls it healthy.

**A report that belongs to an older revision passes the release.** The
canonical release capability names this case:

> #### Scenario: Verified release preparation failure
>
> - **WHEN** a required check fails, is unavailable or belongs to an
>   older revision
> - **THEN** release execution blocks before tag or publication

The release lane's own doctor check honours both clauses: it reports
`stale` for a stale doctor report and `unavailable` for a doctor run
that could not complete, and `check_readiness` blocks on any status that
is not `pass`. Only the DriftWatch check misses them.

**The function's own contract says it should not happen.** The doc
comment above `prepare_release` reads: *"refuses to mark the plan `ready`
when any required check fails, is unavailable or is stale."* The
DriftWatch check can report neither `unavailable` nor `stale` for a
report it did receive.

## What Changes

- **One rule, one place.** The doctor's judgement of a policy report
  becomes a typed verdict the release lane reads, so the two surfaces
  cannot disagree about the same report again.
- **The release check blocks on what the doctor blocks on.** A failing
  rule is a failure, a report with no applicable rule is unavailable
  (never a pass), and a report from an older revision is stale.
- **The evidence states what the report is.** A report that warns is
  still a pass — a warning is not one of the three blocking conditions —
  but its evidence names the warnings instead of calling the report
  healthy. Only a report where every applicable rule passed is called
  healthy.
- **The claims are pinned.** A repository contract check drives the
  release lane with a fake policy adapter and holds it to the four
  verdicts.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The release lane's DriftWatch check | `forge` | Yes | `src/release/engine/prepare.rs`: the status and the evidence it captures. |
| The doctor's policy judgement | `forge` | Yes | `src/doctor/assessment.rs`: the rule becomes a typed verdict both surfaces read. |
| The release lane's doctor check | `forge` | Read only | It already judges with the doctor's own blocking set and already reports `stale` and `unavailable`. |
| The release lane's test check | `forge` | Read only | Unchanged. |
| The release lane's Gate check | `forge` | Read only | Unchanged. |
| The release readiness rule | `forge` | Read only | `check_readiness` blocks on any status that is not `pass`; it is unchanged. |
| The release apply lane | `forge` | Read only | It consumes the plan's checks; a plan that is not ready is refused there already. |
| The policy plane | `forge` | Read only | The report, the severities and the adapter are unchanged. The doctor still re-reads the report. |
| The doctor's policy findings | `forge` | Read only | The per-finding findings and the rollup's wording are unchanged. |
| The `release-publishing` capability | `forge` | Modified | The preparation requirement names the rule the DriftWatch check judges with. |
| The `quality-policy-integration` capability | `forge` | Modified | The delegated execution requirement names every consumer of a policy report. |
| The `release-driftwatch-check-truth` capability | `forge` | Added | this package |
| The release contract version | `forge` | Read only | No field is added, removed or renamed. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §29 (release engine) and §34 (CLI) ask for a gated release; §23/§24 (doctor and the policy plane) own the judgement of a policy report. |
| Contracts | The plan's check `status` changes for three reports (warn, unjudged, stale) and its `evidence`/`detail` wording changes for the warn case. No field is added, removed or renamed, and the contract version is unchanged. |
| Callers | `forge release prepare`, `forge release apply` (it re-captures the checks), the release plan's JSON and human render, the release journal row. |
| Persistence | Nothing new is persisted. The plan's check evidence changes, so the release journal's summary lists the honest check status. |
| Integrations | None. The DriftWatch adapter is invoked exactly as before. |
| Failures | No new error code. |
| Tests | A new `tests/release_driftwatch_check_truth_contract.rs` plus unit tests for the shared verdict. |
| Dependencies | None added. |
| Concerns | Quality: the release lane and the doctor reach one verdict about one report. Compatibility: the plan's shape and its contract version are unchanged; three reports change status, each towards the canonical rule. Security: no new file, process, network or credential surface. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `release-driftwatch-check-truth` | forge | Rust | added | this package |
| `release-publishing` | forge | Rust | modified | the preparation requirement names the rule the DriftWatch check judges with |
| `quality-policy-integration` | forge | Rust | modified | every consumer of a policy report reaches the same verdict |

## Non-Goals

- **The doctor is unchanged.** Its findings, its severities and its
  wording stay exactly as they are; only the rule behind them becomes
  readable by the release lane.
- **The release lane's other checks are unchanged.** The doctor check
  already judges with the doctor's own blocking set; the test and Gate
  checks are untouched.
- **The readiness rule is unchanged.** `check_readiness` still blocks on
  any applicable check that is not `pass`.
- **A warning still does not block a release.** A warning is not one of
  the three conditions the canonical release requirement names, and the
  release lane's own doctor check does not block on a warning either.
  What changes is that the plan stops calling a warninging report
  healthy.
- **The policy adapter is unchanged.** The same binary, the same
  arguments, the same report.
- **The release contract version is unchanged.**
