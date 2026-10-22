## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The project status projection | `forge` | Yes. `src/api/status.rs`. |
| The status projection's own tests | `forge` | Yes. `src/api/status.rs`. |
| The status-evidence contract test | `forge` | Yes. `tests/project_status_evidence_truth_contract.rs` (new). |
| The project-status capability | `forge` | Modified. |
| The `check` sub-check | `forge` | Read only. |
| The readiness sub-check | `forge` | Read only. |
| The overall state reduction | `forge` | Read only. |
| The fleet readiness summary | `forge` | Read only. |
| The workbench health document | `forge` | Read only. |
| The doctor | `forge` | Read only. |
| The release lane | `forge` | Read only. |

## The Gap

### The summary does not name the evidence behind the state

`GET /v1/admin/projects/{id}/status` projects one project's doctor report
into three sub-checks. The doctor sub-check is where the verdict and its
evidence meet:

```rust
// src/api/status.rs
fn doctor_check(report: Option<&DoctorReport>) -> StatusCheck {
    let Some(report) = report else {
        return StatusCheck { id: "doctor", state: STATE_UNAVAILABLE, /* … */ };
    };
    let failing = report
        .findings
        .iter()
        .filter(|finding| finding.applicable && finding.status == FindingStatus::Fail)
        .count();
    let unmet = report.unmet_controls().len();
    if report.healthy {
        StatusCheck {
            id: "doctor",
            state: STATE_HEALTHY,
            summary: format!(
                "{} finding(s), {} unmet control(s); no failing evidence",
                report.findings.len(),
                unmet
            ),
            reason: String::new(),
        }
    } else if report.stale && failing == 0 && unmet == 0 {
        StatusCheck {
            id: "doctor",
            state: STATE_STALE,
            summary: "the registry observation predates the current manifest; re-register to refresh"
                .to_string(),
            reason: String::new(),
        }
    } else {
        StatusCheck {
            id: "doctor",
            state: STATE_ISSUES,
            summary: format!("{failing} failing finding(s), {unmet} unmet control(s)"),
            reason: String::new(),
        }
    }
}
```

`failing` counts `FAIL` only. The branch it feeds is entered whenever the
report is **not healthy**, and the doctor's `healthy` field is:

```rust
// src/doctor/runner.rs
let healthy = findings
    .iter()
    .all(|f| matches!(f.status, FindingStatus::Pass | FindingStatus::Warn))
    && unmet == 0
    && !stale;
```

An `UNAVAILABLE` finding therefore makes the report unhealthy while
contributing **nothing** to the count. The observed projection for a
project whose only problems are two unavailable checks — the repository
inspector cannot run because the project is not a git repository, and the
DriftWatch adapter is not installed on this host:

```text
STATE=issues SUMMARY=0 failing finding(s), 0 unmet control(s)
```

The state says there are issues. The summary says there are none. The
summary is the only place the evidence behind the verdict is named, and
it names nothing.

### The two sub-checks of one response disagree about the same evidence

The `check` sub-check is computed over the **same** doctor report, through
the checker assembly. The checker maps an unavailable finding to a
warning alert:

```rust
// src/checker/mod.rs
let severity = match finding.status {
    FindingStatus::Pass => return None,
    FindingStatus::Fail => AlertSeverity::Error,
    FindingStatus::Warn | FindingStatus::Unavailable => AlertSeverity::Warning,
};
```

So the same two findings are `2 warning(s)` in the `check` sub-check and
`0 failing finding(s)` in the `doctor` sub-check. One response, two
counts, two verdicts about the same evidence.

### The state partition contradicts the doctor's own `healthy` field

The stale branch is entered when `failing == 0 && unmet == 0`, and
`failing` counts `FAIL` only. A report that is stale **and** carries an
unavailable finding is therefore reported `stale`, while the doctor's own
`healthy` field says the report is not healthy.

The canonical capability for this route says something else:

> #### Scenario: Real finding reported honestly
>
> - **WHEN** the in-process doctor or checker reports failing or warning
>   evidence for the project
> - **THEN** that sub-check and the overall state report `issues`, never
>   `healthy` or a pass
>
> #### Scenario: Healthy project
>
> - **WHEN** every sub-check can run and reports no failing or **missing**
>   evidence
> - **THEN** the overall state is `healthy`

An unavailable check is missing evidence, and the codebase's own checker
mapping says it is warning evidence. So the code contradicts the
capability it implements.

### The rest of the codebase already uses the doctor's blocking set

```rust
// src/doctor/model.rs
/// Findings that deny health: every `FAIL`/`UNAVAILABLE`.
pub fn blocking_findings(&self) -> Vec<&Finding> { /* … */ }
```

The release lane blocks on it:

```rust
// src/release/engine/prepare.rs
let blocking = report.blocking_findings();
```

The status projection is the one surface that re-derived the rule with a
narrower filter of its own.

## The Decision

### The projection judges with the doctor's own blocking set

`FAIL` and `UNAVAILABLE` both deny health. The projection uses the
doctor's own set, and its state partition becomes the exact inverse of
the doctor's `healthy` field:

```rust
let blocking = report.blocking_findings();
let failed = blocking
    .iter()
    .filter(|finding| finding.status == FindingStatus::Fail)
    .count();
let unavailable = blocking
    .iter()
    .filter(|finding| finding.status == FindingStatus::Unavailable)
    .count();
let unmet = report.unmet_controls().len();

if !blocking.is_empty() || unmet > 0 {
    // issues
} else if report.stale {
    // stale
} else {
    // healthy
}
```

The doctor's `healthy` field is `no blocking finding && no unmet control
&& !stale`, so the projection's partition and the doctor's field can no
longer disagree.

### The summary names every kind of problem separately

```text
0 failing finding(s), 2 unavailable finding(s), 0 unmet control(s)
```

A check that could not run is missing evidence, not a failing check, and
the two are counted apart. The summary can no longer say "no problems"
while the state says "issues", because the state is derived from exactly
the set the summary counts.

The `healthy` branch's summary keeps its current wording, and the
`stale` branch keeps naming the staleness — a stale report reaches that
branch only when nothing else is wrong.

### A stale report with an unavailable finding reports `issues`

The canonical requirement already demands that for failing or warning
evidence. The staleness is then a second fact about the report, and the
summary names it:

```text
0 failing finding(s), 1 unavailable finding(s), 0 unmet control(s); the registry observation is stale
```

One project's classification changes from `stale` to `issues` — the
classification the requirement demands.

### The claims are pinned

A summary that names a subset of the problems can drift back into
naming none. The contract check holds the projection to its own states:

| Claim | The contract check asserts |
|---|---|
| the state partition | a report with an unavailable finding is `issues`, never `healthy`, `stale` or `unavailable` |
| the summary | it names the failing, the unavailable and the unmet counts, and never says "0 problems" while the state is `issues` |
| the agreement | the doctor sub-check and the check sub-check count the same unavailable findings |
| the doctor's truth | the projection's partition is the inverse of the doctor's own `healthy` field |

## What Is Not Touched, And Why

| Surface | Why it is not in this package |
|---|---|
| The doctor | Its report, its findings, its `healthy` field and its `blocking_findings()` are the truth the projection reads. Nothing here changes what the doctor finds. |
| The `check` sub-check | It already maps an unavailable finding to a warning and counts it. The fix makes the doctor sub-check agree with it. |
| The readiness sub-check | It reads the catalogue's support status, not the doctor. |
| The overall state reduction | `issues > unavailable > stale > healthy` is unchanged. |
| The fleet readiness summary | It counts the states the projection produces; the fix changes one project's classification, and the summary follows. |
| The workbench health document | It carries the whole doctor report, findings included, so it already names every finding. |
| The response's shape | No field is added, removed or renamed, and the contract version is unchanged. The summary stays a bounded prose summary: counts, never finding texts, never a path, never a credential. |
| The doctor's finding inventory | Unchanged. |
| The release lane | It already blocks on `blocking_findings()`. |

## User Experience and Interface

Before, the doctor sub-check of the status route reported a verdict and
an evidence summary that contradicted each other:

```json
{"id": "doctor", "state": "issues", "summary": "0 failing finding(s), 0 unmet control(s)"}
```

while the `check` sub-check, over the same report, named the two findings
that were actually there:

```json
{"id": "check", "state": "issues", "summary": "2 warning(s)"}
```

After, the doctor sub-check names the same evidence, split by kind:

```json
{"id": "doctor", "state": "issues", "summary": "0 failing finding(s), 2 unavailable finding(s), 0 unmet control(s)"}
```

and the two sub-checks agree about the two unavailable checks.

A report with nothing wrong keeps its current summary:

```json
{"id": "doctor", "state": "healthy", "summary": "17 finding(s), 0 unmet control(s); no failing evidence"}
```

and a stale report with nothing else wrong keeps naming the staleness:

```json
{"id": "doctor", "state": "stale", "summary": "the registry observation predates the current manifest; re-register to refresh"}
```

## Behavioral Model

| Situation | Outcome |
|---|---|
| Every finding passes or warns, no control is unmet, the observation is fresh | The sub-check is `healthy` and its summary is unchanged. |
| A finding fails | The sub-check is `issues`; the summary names the failing count, the unavailable count and the unmet count. |
| A finding is unavailable and nothing else is wrong | The sub-check is `issues`; the summary names the unavailable count. **Before**: the same report was `issues` with a summary naming no problem. |
| A control is unmet | The sub-check is `issues`; the summary names the unmet count. |
| The observation is stale and nothing else is wrong | The sub-check is `stale`, unchanged. |
| The observation is stale and a finding is unavailable | The sub-check is `issues` and the summary names the staleness. **Before**: the same report was `stale`, hiding the unavailable finding. |
| The doctor could not run at all | The sub-check is `unavailable` with its safe reason, unchanged. |
| The `check` sub-check | Unchanged: an unavailable finding is a warning alert and is counted. |
| The overall state | Unchanged: `issues` dominates `unavailable`, `stale` and `healthy`. |

## Failure and Boundary Policy

- **A summary never contradicts its own state.** The state is derived
  from exactly the set the summary counts.
- **A missing check is never counted as a failing check.** An unavailable
  finding is counted apart from a failing one.
- **The projection stays bounded.** Counts, never finding texts, never a
  path, never a credential.
- **The projection stays read-only.** It runs the doctor, the checker and
  the catalogue, and writes nothing.
- **The doctor's truth is untouched.** The projection reads it; it never
  decides what the doctor found.

## Verification Oracle

```bash
# The Forge Core test suite
cargo test --workspace --all-targets -- \
  --skip rust_scaffold_builds_and_tests_with_native_toolchain \
  --skip rust_scaffold_builds_with_native_toolchain_without_forge \
  --skip declaration_is_inert_for_native_builds_present_and_absent \
  --test-threads=1

# The repository Gate
./target/debug/forge gate --dry-run
./target/debug/forge gate --timeout-secs 1800

# Strict OpenSpec validation and whitespace
openspec validate --all --strict --no-interactive
node scripts/check-openspec-change-names.mjs
git diff --check
```

Three real runs:

1. The status route for a project whose only problems are unavailable
   checks: the doctor sub-check names the unavailable count, and the
   `check` sub-check names the same findings.
2. The status route for a healthy project: the `healthy` summary is
   unchanged.
3. The fleet summary: it counts the states the projection produces.

## Implementation Plan

**The projection.** `src/api/status.rs`: the doctor sub-check judges with
`blocking_findings()`, separates the failing count from the unavailable
count, and names the staleness when the report carries it.

**The tests.** `src/api/status.rs` carries the rule's unit tests;
`tests/project_status_evidence_truth_contract.rs` pins the projection
against the doctor's own `healthy` field and against the sibling
sub-check.

## Decision Ledger

- **Resolved:** the projection judges with the doctor's own blocking set,
  so its state partition is the exact inverse of the doctor's `healthy`
  field.
- **Resolved:** the summary names the failing, the unavailable and the
  unmet counts apart; a missing check is never counted as a failing one.
- **Resolved:** a stale report with an unavailable finding reports
  `issues` and names the staleness, because the canonical requirement
  already demands `issues` for failing or warning evidence.
- **Resolved:** the `check` sub-check is unchanged and the two sub-checks
  now agree.
- **Resolved:** the response's fields, the four states and the contract
  version are unchanged; only the summary's wording and one project's
  classification move.
- **Resolved:** the doctor is untouched; the projection reads it.
- **Implementation authorized (2026-10-22):** the owner directed the
  trunk repair programme and authorized continuing it after
  `profile-toolchain-version-truth` closed the toolchain-version gap.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| A status projection names the evidence behind its verdict | The summary can no longer say "no problems" while the state says "issues" | The unavailable count is named | The blocking set | `src/api/status.rs` | The summary names every kind of problem | 3.1, 3.3 | The contract test; the real runs |
| The projection and the doctor agree | The partition is the inverse of the doctor's `healthy` | The state is `issues` for an unavailable finding | The inverse partition | `src/api/status.rs` | An unavailable finding is `issues`, never `healthy` or `stale` | 3.1, 3.2 | The contract test |
| The two sub-checks agree | The doctor sub-check counts what the check sub-check counts | The same two findings, named by both | The shared rule | `src/api/status.rs` | Both count the unavailable findings | 3.2, 3.3 | The contract test |
| The claims are pinned | The projection cannot drift back to naming a subset | The claims are held | The contract check | `tests/project_status_evidence_truth_contract.rs` | The rule is held | 3.4 | The contract test |
