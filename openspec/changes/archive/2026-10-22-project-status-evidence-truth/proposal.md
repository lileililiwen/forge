# Change: project-status-evidence-truth

## Why

`GET /v1/admin/projects/{id}/status` reports three sub-checks — `doctor`,
`check` and `readiness` — and an overall state. The doctor sub-check's
summary does not name the evidence behind its own state.

```rust
// src/api/status.rs
let failing = report
    .findings
    .iter()
    .filter(|finding| finding.applicable && finding.status == FindingStatus::Fail)
    .count();
let unmet = report.unmet_controls().len();
if report.healthy {
    // healthy
} else if report.stale && failing == 0 && unmet == 0 {
    // stale
} else {
    StatusCheck {
        id: "doctor",
        state: STATE_ISSUES,
        summary: format!("{failing} failing finding(s), {unmet} unmet control(s)"),
        reason: String::new(),
    }
}
```

`failing` counts only `FAIL`. But the branch is entered whenever the
report is **not healthy**, and the doctor's own `healthy` field is

```rust
let healthy = findings
    .iter()
    .all(|f| matches!(f.status, FindingStatus::Pass | FindingStatus::Warn))
    && unmet == 0
    && !stale;
```

so an `UNAVAILABLE` finding makes the report unhealthy while contributing
nothing to the count. A project that is not a git repository, or a host
without the DriftWatch adapter, is reported like this:

```text
STATE=issues SUMMARY=0 failing finding(s), 0 unmet control(s)
```

The state says there are issues. The summary says there are none. The
summary is the only place the evidence behind the verdict is named, and
it names nothing.

**The two sub-checks of one response disagree about the same evidence.**
The `check` sub-check is computed over the *same* doctor report through
the checker assembly, and the checker maps an unavailable finding to a
warning alert:

```rust
// src/checker/mod.rs
FindingStatus::Warn | FindingStatus::Unavailable => AlertSeverity::Warning,
```

So the same two findings are `2 warning(s)` in the `check` sub-check and
`0 failing finding(s)` in the `doctor` sub-check. One response, two
counts, two verdicts.

**The state partition contradicts the doctor's own `healthy` field.** The
stale branch is entered when `failing == 0 && unmet == 0`, and `failing`
counts only `FAIL`. A report that is stale *and* carries an unavailable
finding is therefore reported `stale` — while the doctor's own `healthy`
field says the report is not healthy, and the canonical requirement says
something else:

> #### Scenario: Real finding reported honestly
>
> - **WHEN** the in-process doctor or checker reports failing or warning
>   evidence for the project
> - **THEN** that sub-check and the overall state report `issues`, never
>   `healthy` or a pass

An unavailable finding is warning evidence — the checker's own mapping
says so. So the code contradicts the capability it implements.

The rest of the codebase already uses the doctor's own blocking set.
`DoctorReport::blocking_findings()` is `FAIL` or `UNAVAILABLE`, and the
release lane blocks on it:

```rust
// src/doctor/model.rs
pub fn blocking_findings(&self) -> Vec<&Finding> {
    self.findings
        .iter()
        .filter(|f| matches!(f.status, FindingStatus::Fail | FindingStatus::Unavailable))
        .collect()
}
```

The status projection is the one surface that re-derived the rule with a
narrower filter of its own.

## What Changes

- **The doctor sub-check judges with the doctor's own blocking set.** A
  finding that is `FAIL` or `UNAVAILABLE` blocks, exactly as
  `blocking_findings()` defines, and the projection's state partition
  becomes the exact inverse of the doctor's `healthy` field.
- **The summary names the evidence behind the verdict.** It reports the
  failing findings, the unavailable findings and the unmet controls
  separately, so a check that could not run is never counted as a
  failing check and the summary can never say "no problems" while the
  state says "issues".
- **A stale report with an unavailable finding reports `issues`.** The
  canonical requirement already demands that for failing or warning
  evidence; the staleness is then named in the summary rather than
  carrying the whole verdict on its own.
- **The claims are pinned.** A repository contract check holds the
  projection to its own states, so a summary cannot drift back into
  naming a subset of the problems.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The project status projection | `forge` | Yes | `src/api/status.rs`: the doctor sub-check's state rule and summary. |
| The status projection's own tests | `forge` | Yes | `src/api/status.rs`: the rule's unit tests. |
| The status-evidence contract test | `forge` | Yes | `tests/project_status_evidence_truth_contract.rs`. |
| The project-status capability | `forge` | Modified | The requirement names the blocking set the sub-check judges with. |
| The `project-status-evidence-truth` capability | `forge` | Added | this package |
| The `check` sub-check | `forge` | Read only | It already counts an unavailable finding as a warning. |
| The readiness sub-check | `forge` | Read only | It reads the catalogue, not the doctor. |
| The overall state reduction | `forge` | Read only | `issues > unavailable > stale > healthy` is unchanged. |
| The fleet readiness summary | `forge` | Read only | It counts the states the projection produces. |
| The workbench health document | `forge` | Read only | It carries the whole doctor report, findings included. |
| The doctor itself | `forge` | Read only | Its report, its `healthy` field and its findings are unchanged. |
| The release lane | `forge` | Read only | It already blocks on `blocking_findings()`. |
| The API contract version | `forge` | Read only | No field is added, removed or renamed. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §23 asks the doctor to inspect a project; the project-status capability is the repository's own canonical contract for this route. |
| Contracts | The response's fields, the four states and the contract version are unchanged. The `summary` prose changes, and one project's classification changes from `stale` to `issues` — the classification the canonical requirement already demands. |
| Callers | The portal's project tile, the fleet tile and the fleet summary read the route. |
| Persistence | Nothing is persisted. |
| Integrations | None. No new toolchain, network or credential surface. |
| Failures | No new error code. |
| Tests | A new `tests/project_status_evidence_truth_contract.rs` plus unit tests in `src/api/status.rs`. |
| Dependencies | None added. |
| Concerns | Quality: a status projection names the evidence behind its own verdict, and it stops contradicting the doctor it projects. Compatibility: the response's shape and its states are unchanged. Security: no new file, process, network or credential surface. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `project-status-evidence-truth` | forge | Rust | added | this package |
| `forge-web-project-status` | forge | Rust | modified | the sub-check requirement names the blocking set it judges with |

## Non-Goals

- **The doctor is unchanged.** Its report, its findings, its `healthy`
  field and its `blocking_findings()` are the truth the projection reads.
- **The `check` sub-check is unchanged.** It already counts an
  unavailable finding as a warning, and the fix makes the doctor sub-check
  agree with it.
- **The overall state reduction is unchanged.** `issues` still dominates
  `unavailable`, `stale` and `healthy`.
- **The workbench's health document is unchanged.** It carries the whole
  doctor report, so it already names every finding.
- **The doctor sub-check still surfaces counts, not texts.** The route
  stays a bounded projection: it names how many findings of each kind
  there are, never the finding texts, the project's path or a credential.
- **The API contract version is unchanged.** No field is added, removed or
  renamed.
- **The doctor's finding inventory is unchanged.**
