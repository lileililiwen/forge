# Change: doctor-finding-applicability

## Why

`forge doctor` reports two things about a project: a **finding
inventory** and a **maturity control list**. The controls are target-gated
— a control applies only when the project reaches the level that declares
it. The findings are not.

Three findings mirror an L2 control and warn about it whatever the
project's target maturity is:

```rust
// src/doctor/runner.rs, before this change
let (deploy_present, deploy_evidence) = detect_deployment(dir);
findings.push(Finding::new(
    "deployment-config",
    if deploy_present { FindingStatus::Pass } else { FindingStatus::Warn },
    deploy_evidence,
    true,                              // <- always applicable
    Remediation::Manual,
    if deploy_present {
        "deployment configuration is present"
    } else {
        "no deployment configuration; required for L2 deployment"
    },
));
```

The same shape appears for `ci-config` (the L2-ci control) and for
`driftwatch-config` (the L2-driftwatch control). So a project that targets
L1 is warned about the L2 configurations it does not need — while the same
report says the control does not apply:

```console
$ forge doctor ./my-l1-project
  [WARN] driftwatch-config (automatic)
    evidence: no driftwatch configuration
    detail: no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3)
```

and the maturity control list of that same report says:

```text
L2-driftwatch | applicable: false | met: false | ['no driftwatch configuration']
```

One report, two answers about the same requirement: the finding warns
about it, the control says it does not apply. The canonical capability
promises one rule for both:

> Read-only maturity assessment through forge doctor: a stable finding
> inventory (PASS/WARN/FAIL/UNAVAILABLE) with evidence and remediation
> classes, **target-gated L0–L4 applicability**, …

The release lane already reads the requirement the other way, from the
project's own declaration:

```console
$ forge release prepare ./my-l1-project --version 0.1.0
  - driftwatch disabled applicable=false …: check `driftwatch` is not configured; release does not require it
```

So the release lane says "not required", the doctor's control says "does
not apply", and the doctor's finding says "required for L2" — and the
warning does not stay inside the doctor. The checker plane turns it into
a machine alert for a CI gate to read:

```console
$ forge check ./my-l1-project
{"alerts":[{"severity":"warning","message":"driftwatch-config: no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3); no driftwatch configuration","source":"doctor","symbol":"doctor/driftwatch-config"}]}
```

Every project Forge generates is in that state: the scaffolds declare
`maturity: L1` and no DriftWatch configuration, so every fresh project
carries a warning it has not asked for.

## What Changes

- **The doctor's configuration findings follow the same applicability rule
  as the controls they mirror.** `ci-config` and `deployment-config`
  apply when the project's target maturity reaches L2, the level that
  declares the L2-ci and L2-deployment controls. `driftwatch-config`
  applies when the target maturity reaches L2 **or** when the manifest's
  release checks declare a DriftWatch check, which is the rule the
  release lane already applies.
- **A configuration the project does not require is reported as not
  applicable, not as a warning.** The finding still reports the absence
  as its own state — the canonical contract requires the absence to be
  reported as a distinct state from a configured-but-failing run — and no
  surface renders it as a warning any more.
- **The report stops contradicting itself.** The finding and the control
  that mirrors it now answer the same question the same way, and the
  checker plane stops emitting an alert for a requirement the project
  does not have.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The doctor's finding inventory | `forge` | Yes | `src/doctor/runner.rs`: the applicability of `ci-config`, `deployment-config` and `driftwatch-config`. |
| The doctor's maturity controls | `forge` | Read only | `src/doctor/assessment.rs` already applies the target-gated rule; the change makes the findings agree with it. |
| The checker plane | `forge` | Read only | `forge check` renders the doctor's findings; it stops emitting the warning alert. |
| The release lane | `forge` | Read only | `src/release/` already reads the requirement from the manifest; the doctor now agrees with it. |
| The doctor's contract test | `forge` | Read only | `tests/doctor_contract.rs` keeps its assertions. |
| The DriftWatch contract tests | `forge` | Yes (assertions updated) | `tests/driftwatch_cli_contract.rs` and `tests/driftwatch_cli_alignment_contract.rs` pinned the old warning for a project that targets L1 and declares no DriftWatch check; they now pin the not-applicable state and keep asserting that the evidence names the absence. |
| The checker contract test | `forge` | Yes (fixture updated) | `tests/checker_contract.rs` bounded the checker document's alert truncation with a project whose dropped configurations warned; its fixture now targets L2, where the three configurations are required and warn. |
| The finding-applicability contract test | `forge` | Yes | `tests/doctor_finding_applicability_contract.rs`: the three findings against the three target maturities. |
| The doctor unit tests | `forge` | Yes | `src/doctor/doctor_tests.rs` gains the case-level checks. |
| The manifest | `forge` | Read only | The release-checks list is read; no field is added, renamed or defaulted. |
| The profile catalogue | `forge` | Read only | The maturity levels are unchanged. |
| The `doctor-maturity-assessment` capability | `forge` | Modified | The finding inventory states the shared applicability rule. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md:962` (§23 Doctor) lists the inspections the doctor performs; `requirement.md:1046` (§25 Maturity Model) makes the level's requirements conditional on what the project declares. |
| Contracts | The finding IDs, the status set and the evidence shape are unchanged. A finding's `applicable` flag and its status change for the projects that do not require the configuration. |
| Callers | `forge doctor` (CLI), the MCP doctor method, the API route, the checker document and the release lane all read the same findings. |
| Persistence | Nothing is persisted. |
| Integrations | None. No new toolchain, network or credential surface. The doctor stays read-only. |
| Failures | No new error code. A required configuration that is missing keeps its warning. |
| Tests | A new `tests/doctor_finding_applicability_contract.rs` pins the applicability of the three findings; the existing doctor tests keep passing. |
| Dependencies | None added. |
| Concerns | Quality and compatibility: the findings and the controls now share one rule, and the shared rule is pinned by a contract check. Security: no new file, process, network or credential surface. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `doctor-finding-applicability` | forge | Rust | added | this package |
| `doctor-maturity-assessment` | forge | Rust | modified | the finding inventory states that a finding and the control it mirrors share one applicability rule |

## Non-Goals

- **The doctor does not stop reporting the absence.** The canonical
  contract requires the absence of a DriftWatch configuration to be a
  distinct state from a configured-but-failing run, so the finding stays
  and keeps its evidence.
- **The maturity controls are untouched.** They already apply the
  target-gated rule; this change makes the findings agree with them.
- **The release lane is untouched.** It already reads the requirement
  from the manifest.
- **A project that requires the configuration keeps the warning.** A
  project targeting L2, or one whose manifest declares the DriftWatch
  check, is warned when the configuration is missing, exactly as today.
- **The findings' evidence and remediation class do not change.** Only
  the applicability and the status of a configuration the project does
  not require.
- **The doctor does not judge the current maturity differently.** The
  controls are assessed against the current and the target levels, as
  today; this change is about the findings' applicability.
- **The doctor's other findings are untouched.** The manifest, profile,
  features, dependency drift, build configuration, repository,
  documentation, workspace metadata, declaration vocabulary, policy,
  gate evidence, release evidence, registry observation and delivery
  findings keep their behaviour.
