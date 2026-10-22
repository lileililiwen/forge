## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The doctor's finding inventory | `forge` | Yes. `src/doctor/runner.rs`. |
| The finding-applicability contract test | `forge` | Yes. `tests/doctor_finding_applicability_contract.rs`. |
| The doctor unit tests | `forge` | Yes. `src/doctor/doctor_tests.rs`. |
| The DriftWatch contract tests | `forge` | Yes (assertions updated). `tests/driftwatch_cli_contract.rs`, `tests/driftwatch_cli_alignment_contract.rs`. |
| The checker contract test | `forge` | Yes (fixture updated). `tests/checker_contract.rs`. |
| The doctor's maturity controls | `forge` | Read only. `src/doctor/assessment.rs`. |
| The checker plane | `forge` | Read only. It renders the findings. |
| The release lane | `forge` | Read only. `src/release/`. |
| The manifest | `forge` | Read only. The release-checks list is read. |

## The Gap

**The controls are target-gated. The findings are not.**

The maturity controls take their applicability from the level that
declares them:

```rust
// src/doctor/assessment.rs
control(
    "L2-driftwatch",
    Maturity::L2,
    "driftwatch is configured; execution belongs to v0.3 (L2 DriftWatch)",
    at_least(Maturity::L2),   // <- the applicability
    dw_present,
    dw_evidence,
),
```

The three findings that mirror those controls pass `true` for
applicability, whatever the project's target maturity is:

```rust
// src/doctor/runner.rs, before this change
findings.push(Finding::new(
    "driftwatch-config",
    if dw_present { FindingStatus::Pass } else { FindingStatus::Warn },
    dw_evidence,
    true,                 // <- always applicable
    Remediation::Automatic,
    if dw_present {
        "driftwatch configuration is present"
    } else {
        "no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3)"
    },
));
```

The same shape is used for `deployment-config` (the L2-deployment
control) and for `ci-config` (the L2-ci control).

**So one report gives two answers about the same requirement.**

```console
$ forge doctor ./my-l1-project
  [WARN] driftwatch-config (automatic)
    evidence: no driftwatch configuration
    detail: no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3)
```

```text
L2-driftwatch | applicable: false | met: false | ['no driftwatch configuration']
```

The finding warns about a requirement; the control that declares the same
requirement says the project does not have it. The canonical capability
promises one rule for both:

> Read-only maturity assessment through forge doctor: a stable finding
> inventory (PASS/WARN/FAIL/UNAVAILABLE) with evidence and remediation
> classes, **target-gated L0–L4 applicability**, …

**The release lane reads the requirement from the project, and says so.**

```console
$ forge release prepare ./my-l1-project --version 0.1.0
  - driftwatch disabled applicable=false …: check `driftwatch` is not configured; release does not require it
```

The release lane asks the project's own declaration. The doctor asks
nothing and warns.

**And the warning leaves the doctor.** The checker plane renders the
finding as an alert for a CI gate:

```console
$ forge check ./my-l1-project
{"alerts":[{"severity":"warning","message":"driftwatch-config: no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3); no driftwatch configuration","source":"doctor","symbol":"doctor/driftwatch-config"}]}
```

Every project Forge generates is in that state: the scaffolds declare
`maturity: L1` and ship no DriftWatch configuration, so every fresh
project carries a warning about a level it did not ask for.

## The Decision

### One applicability rule, shared with the control each finding mirrors

| Finding | Control it mirrors | The project requires the configuration when |
|---|---|---|
| `ci-config` | `L2-ci` | the target maturity is at least L2 |
| `deployment-config` | `L2-deployment` | the target maturity is at least L2 |
| `driftwatch-config` | `L2-driftwatch` | the target maturity is at least L2, **or** the manifest's release checks declare a DriftWatch check |

The DriftWatch row has a second condition because the release lane has
one: the manifest's `release.checks` may declare a `driftwatch` check at
any maturity, and then the project does require the configuration — the
release lane says so in its own words ("release does not require it" is
the opposite case). The doctor's finding and the release lane's row now
read the same project the same way.

The target maturity is the gate because that is the level the controls
are assessed against. A project that targets L2 is warned about the
missing L2 configuration, exactly as today; a project that targets L1 is
not.

### A configuration the project does not require is not applicable, not a warning

The finding stays. The canonical contract requires the absence to be
reported as a distinct state:

> #### Scenario: No markers
>
> - **WHEN** a project contains none of the recognized names
> - **THEN** doctor reports no DriftWatch configuration as a distinct
>   state from a configured-but-failing run

So the finding keeps its evidence ("no driftwatch configuration") and its
identity. What changes is that it is not applicable when the project has
no such requirement, and the house's not-applicable shape is used — the
same shape the workspace-metadata and declaration-vocabulary findings
already use:

```rust
Finding::new(
    "driftwatch-config",
    FindingStatus::Pass,
    evidence,
    false,          // <- not applicable: the project does not require it
    Remediation::Automatic,
    "the project does not require a DriftWatch configuration at its target maturity",
)
```

The report then reads:

```text
  [PASS] driftwatch-config (automatic)
    evidence: no driftwatch configuration
    detail: the project does not require a DriftWatch configuration at its target maturity
    applicability: not applicable
```

and the checker plane emits no alert for it.

**A project that requires the configuration is warned, as today.** A
project targeting L2, or one whose manifest declares the DriftWatch check,
gets the warning with the same evidence and the same remediation class.

### The shared rule is pinned

A rule that only one finding follows is a convention. The contract check
asserts the applicability of the three findings across the three
maturities a project can target:

| Project | `ci-config` | `deployment-config` | `driftwatch-config` |
|---|---|---|---|
| target L1, no release check | not applicable | not applicable | not applicable |
| target L2 | applicable | applicable | applicable |
| target L1, manifest declares the driftwatch check | not applicable | not applicable | applicable |

and the check reads the rule from the catalogue's maturity levels rather
than restating them.

## What Is Not Touched, And Why

| Surface | Why it is not in this package |
|---|---|
| The maturity controls (`src/doctor/assessment.rs`) | They already apply the target-gated rule. This change makes the findings agree with them, not the other way round. |
| The release lane (`src/release/`) | It already reads the requirement from the manifest and reports the row as not applicable. |
| The checker plane | It renders the doctor's findings. It stops emitting the alert because the finding is no longer a warning. |
| The DriftWatch marker probe (`detect_driftwatch`) | The set of recognized markers is unchanged; the finding still reports the absence. |
| The policy findings (`driftwatch-policy` and the per-rule findings) | They are the policy adapter's own findings, not a configuration requirement. They stay as they are. |
| The manifest | The release-checks list is read; no field is added, renamed or defaulted. |
| The profile catalogue | The maturity levels are unchanged. |
| The doctor's other findings | The manifest, profile, features, dependency drift, build configuration, repository, documentation, workspace metadata, declaration vocabulary, gate evidence, release evidence, registry observation and delivery findings keep their behaviour. |
| The three existing contract tests that pinned the old warning | They encoded the behaviour this change removes, so their assertions are updated: the two DriftWatch contract tests now pin the not-applicable state (and keep asserting that the evidence names the absence), and the checker contract test's fixture now targets L2, where the three dropped configurations are required and warn, so the test still bounds the alert truncation. |
| The doctor's read-only promise | Nothing new is read and nothing is written. |

## User Experience and Interface

Before, a project that targets L1 was warned about a level it did not
ask for, and the checker plane turned that warning into an alert:

```console
$ forge doctor ./my-l1-project
  [WARN] driftwatch-config (automatic)
    evidence: no driftwatch configuration
    detail: no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3)
```

```console
$ forge check ./my-l1-project
{"alerts":[{"severity":"warning","message":"driftwatch-config: …","source":"doctor","symbol":"doctor/driftwatch-config"}]}
```

After, the absence is reported as what it is — a state the project does
not need — and the report agrees with its own control list:

```console
$ forge doctor ./my-l1-project
  [PASS] driftwatch-config (automatic)
    evidence: no driftwatch configuration
    detail: the project does not require a DriftWatch configuration at its target maturity
    applicability: not applicable
```

A project that does require it is still warned:

```console
$ forge doctor ./my-l2-project
  [WARN] driftwatch-config (automatic)
    evidence: no driftwatch configuration
    detail: no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3)
```

## Behavioral Model

| Situation | Outcome |
|---|---|
| A project targets L1 and declares no driftwatch check | `driftwatch-config` is not applicable; the absence is still reported; no checker alert. |
| A project targets L2 without a DriftWatch configuration | `driftwatch-config` warns, as today; the L2-driftwatch control is unmet. |
| A project targets L1 and declares the driftwatch check in `release.checks` | `driftwatch-config` warns, as today — the release lane requires it. |
| A project targets L1 and has a DriftWatch configuration | `driftwatch-config` passes, as today. |
| A project targets L1 without a Dockerfile | `deployment-config` is not applicable; the absence is still reported. |
| A project targets L2 without a Dockerfile | `deployment-config` warns, as today. |
| A project targets L1 without CI | `ci-config` is not applicable; the absence is still reported. |
| A project targets L2 without CI | `ci-config` warns, as today. |
| The policy plane cannot run | Unchanged: the policy findings stay `UNAVAILABLE`. |
| A project with a manifest that declares no release checks | Unchanged: the release lane's rows are read from the manifest as they are today. |

## Failure and Boundary Policy

- **A required configuration that is missing keeps its warning.** The
  change never softens a requirement the project has.
- **The absence is never hidden.** The finding stays, with its evidence.
  Only its applicability and its status change.
- **The rule is the project's own.** The target maturity and the
  manifest's release checks are the project's declarations; the doctor
  reads them and does not invent a requirement.
- **The check is read-only.** Nothing in this package writes to the
  inspected project.
- **The applicability rule is pinned.** A change to the maturity levels
  or to the rule fails the contract check, naming the finding.

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

1. `forge doctor` on a project that targets L1: the three configuration
   findings are not applicable, and the report agrees with its own
   control list.
2. `forge doctor` on a project that targets L2 without the
   configurations: the three findings warn, as before.
3. `forge check` on the L1 project: no configuration alert.

## Implementation Plan

**The findings.** `src/doctor/runner.rs`: the applicability of
`ci-config`, `deployment-config` and `driftwatch-config` follows the rule
above; the finding keeps its evidence, its remediation class and its
identity.

**The rule.** The target maturity and the manifest's release checks are
already in the runner's scope: the runner holds the manifest and the
resolved target maturity.

**The tests.** `tests/doctor_finding_applicability_contract.rs` pins the
applicability of the three findings across the maturities;
`src/doctor/doctor_tests.rs` gains the case-level checks.

## Decision Ledger

- **Resolved:** the three configuration findings apply the same rule as
  the controls they mirror — the target maturity reaches L2.
- **Resolved:** `driftwatch-config` also applies when the manifest's
  release checks declare a DriftWatch check, because the release lane
  requires the configuration then and says so.
- **Resolved:** a configuration the project does not require is reported
  as not applicable, with the absence still in the evidence. The finding
  is not removed.
- **Resolved:** the policy findings are untouched. They are the policy
  adapter's findings, not a configuration requirement.
- **Resolved:** the maturity controls, the release lane and the checker
  plane are untouched; they read the change.
- **Implementation authorized (2026-10-22):** the owner directed the
  trunk repair programme and authorized this package in the approved
  order, after `profile-build-command-runnable` closed the declared
  build command gap.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| The doctor's findings and controls share one applicability rule | A project is not warned about a level it does not target | The report agrees with its own control list | The target-gated rule | `src/doctor/runner.rs` | L1 is not applicable; L2 warns | 3.1, 3.4 | The contract test; the real runs |
| A configuration the project does not require is reported, not warned | The absence stays visible | The finding is not applicable | The house's not-applicable shape | `src/doctor/runner.rs` | The evidence still names the absence | 3.1 | The contract test |
| The DriftWatch requirement is the project's own | The doctor agrees with the release lane | The manifest decides | The manifest's release checks | `src/doctor/runner.rs` | A declared driftwatch check warns at L1 | 3.1, 3.4 | The contract test |
| The finding inventory states the shared rule | The contract names the rule | The report is consistent | `doctor-maturity-assessment` gains a scenario | `src/doctor/runner.rs` | The findings and the controls agree | 3.1 | The contract test |
