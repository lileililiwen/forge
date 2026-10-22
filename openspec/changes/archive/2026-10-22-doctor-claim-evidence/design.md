## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The doctor's finding inventory | `forge` | Yes. `src/doctor/runner.rs`. |
| The doctor's maturity controls | `forge` | Yes. `src/doctor/assessment.rs`. |
| The CLI's registry observation | `forge` | Yes. `src/cli/projects.rs`. |
| The MCP method's registry observation | `forge` | Yes. `src/mcp/params.rs`. |
| The claim-evidence contract test | `forge` | Yes. `tests/doctor_claim_evidence_contract.rs`. |
| The doctor unit tests | `forge` | Yes. `src/doctor/doctor_tests.rs`. |
| The checker plane | `forge` | Read only. It renders the findings. |
| The release lane | `forge` | Read only. |
| The API surfaces | `forge` | Read only. |
| The profile catalogue | `forge` | Read only. |
| The manifest | `forge` | Read only. |

## The Gap

### The registry-observation finding states a fact the doctor cannot know

```rust
// src/doctor/runner.rs, before this change
match observation {
    Some(obs) if obs.registered => {
        // fresh or stale
    }
    _ => findings.push(Finding::new(
        "registry-observation",
        FindingStatus::Warn,
        vec!["project is not registered; maturity evidence is local-only".to_string()],
        true,
        Remediation::Automatic,
        "project is not registered",
    )),
}
```

Two different situations fall into that `_` arm:

| Situation | What the doctor knows | What it says |
|---|---|---|
| The caller supplied an observation that says the project is not registered | the project is not registered | correct |
| The caller supplied no observation at all | nothing | "project is not registered" |

The doctor never opens the registry — its own module contract says so:

> Registry access, when the caller supplies it, is limited to a read-only
> observation used to mark stale results as stale.

And the sibling delivery finding already states the house's rule right
below the same block:

> The caller supplies the observation; the doctor never opens the
> registry itself. A caller that did not look produces no finding.

The release lane is the caller that does not look: it resolved the
project from the registry and then ran the doctor with `None`, so its own
doctor report says the project it just found is not registered.

### The callers cannot say which situation they are in

```rust
// src/cli/projects.rs and src/mcp/params.rs, before this change
fn observation_for(registry: &Registry, path: &Path) -> Option<RegistryObservation> {
    let canonical = path.canonicalize().ok()?.display().to_string();
    let record = registry
        .list()
        .ok()?
        .into_iter()
        .find(|p| p.path == canonical)?;
    ...
}
```

The `?` on `find` returns `None` when the registry holds no record for
the path — the same value the function returns when the registry was
never consulted at all. The two callers that consult the registry
read-only, the CLI's `forge doctor` and the MCP `run_doctor` method,
therefore cannot report "this project is not registered" in a way the
doctor can distinguish from silence.

That is why the fix has two halves. The doctor learns to read the
difference between an absent observation and an observation that says
"not registered", and the callers learn to say which one they have.

### The L3-registry control states the same fact the doctor cannot know

```rust
// src/doctor/assessment.rs, before this change
let registered = observation.map(|o| o.registered).unwrap_or(false);
...
control(
    "L3-registry",
    Maturity::L3,
    "project is registered (L3 registry)",
    at_least(Maturity::L3),
    registered,
    vec![if registered {
        "project is registered".to_string()
    } else {
        "project is not registered".to_string()
    }],
)
```

With no observation the control claims the project is not registered and
counts as an unmet L3 control, so the maturity-requirements finding fails
the project for a fact the doctor never read. Leaving the control alone
while fixing the finding would make one report contradict itself.

### The L1-structure control claims a comparison that never happens

```text
  [PASS] L1-structure (L1): project structure matches the profile layout (L1 structure)
    evidence: forge.yaml and profile build definition are present
```

The control's implementation:

```rust
let structure_met = build_present && file_exists(dir, "forge.yaml");
```

Two checks. The profile's layout is never read, by the doctor or by any
other surface — `src/profile/model.rs::layout` is read by `forge profile
inspect` and by the catalogue's own unit test, and by nothing else.

And the claim is false for four of the six supported profiles. The
declared layouts and the generated trees:

| Profile | Declared layout | In the generated project |
|---|---|---|
| `python-service` | `app/`, `tests/`, `pyproject.toml`, `Dockerfile` | all four |
| `rust-web` | `src/`, `tests/`, `Cargo.toml` | `tests/` missing |
| `react-web` | `src/`, `public/`, `index.html`, `package.json`, `vite.config.js`, `tests/` | `public/`, `tests/` missing |
| `flutter-app` | `lib/`, `test/`, `pubspec.yaml`, `android/`, `ios/` | `android/`, `ios/` missing |
| `aspnet-web` | `src/`, `tests/`, `Dockerfile`, `appsettings.json` | `src/`, `tests/` missing |

So the doctor tells the operator that a generated project's structure
matches its profile's declared structure, and a directory listing of that
project shows that it does not.

### The dependency-drift finding credits evidence the check did not read

```rust
// src/doctor/runner.rs, before this change
let expected_lang = descriptor.as_ref().map(|d| d.language.as_str()).unwrap_or("");
let manifest_lang = manifest.runtime.as_ref().and_then(|r| r.language.clone()).unwrap_or_default();
let mut drift: Vec<String> = Vec::new();
if !manifest_lang.is_empty() && manifest_lang != expected_lang {
    drift.push(format!("manifest runtime language '{manifest_lang}' disagrees with profile language '{expected_lang}'"));
}
if let BuildDefinitionOutcome::Missing(evidence) = &build_definition {
    drift.push(evidence.join("; "));
}
if drift.is_empty() {
    findings.push(Finding::new(
        "dependency-drift",
        FindingStatus::Pass,
        vec!["manifest runtime agrees with on-disk dependency evidence".to_string()],
        ...
```

The check compares the manifest's runtime language with the **profile's**
language and asks whether the profile's build definition file is present.
No language marker is read from the project. The evidence names on-disk
dependency evidence the check never consulted.

The comparison is also skipped when the manifest declares no runtime
language, so an honest evidence line has to say that too rather than
claim a comparison that did not happen.

## The Decision

### The doctor states what it knows and what it checked

Four claims, one rule: **a finding and a control state only what the
doctor read.**

#### The registry observation

The doctor keeps three states, and the observation carries all three:

| Observation | Finding |
|---|---|
| `Some`, registered, fresh | `PASS` — "registry observation is current" |
| `Some`, registered, stale | `WARN` — the observation predates the manifest |
| `Some`, not registered | `WARN` — "project is not registered; maturity evidence is local-only" |
| `None` | `PASS`, not applicable — "no registry observation was supplied; the registration state is unknown to the doctor" |

The unknown state uses the house's not-applicable shape from
`doctor-finding-applicability`:

```rust
Finding::new(
    "registry-observation",
    FindingStatus::Pass,
    vec!["no registry observation was supplied; the registration state is unknown to the doctor".to_string()],
    false,          // not applicable
    Remediation::Automatic,
    "no registry observation was supplied; the doctor does not open the registry itself",
)
```

The finding's identity, its remediation class and its place in the
inventory stay. The staleness rule is untouched: an observation that
predates the manifest is still reported as stale, and an observation that
says the project is not registered still warns.

The doctor keeps the difference between *not registered* and *not
looked*, and the release lane's doctor report no longer contradicts the
release lane's own registry lookup.

#### The callers say which situation they are in

`observation_for` in the CLI and in the MCP parameters answers the
question the doctor's three states need:

```rust
fn observation_for(registry: &Registry, path: &Path) -> Option<RegistryObservation> {
    let canonical = path.canonicalize().ok()?.display().to_string();
    let records = registry.list().ok()?;
    let Some(record) = records.into_iter().find(|p| p.path == canonical) else {
        // The registry was consulted and holds no record for this path.
        return Some(RegistryObservation {
            registered: false,
            observed_at: None,
            delivery: None,
        });
    };
    ...
}
```

The CLI's and the MCP method's behaviour is unchanged for the operator:
a project that is not in the registry still warns that it is not
registered. What changes is that the warning is now backed by a fact
the caller actually read.

A registry that cannot be opened or listed still yields no observation,
and the doctor reports the unknown state — which is true, because in
that case the caller has no answer to give.

#### The L3-registry control

The control follows the observation:

```rust
let registration_known = observation.is_some();
let registered = observation.map(|o| o.registered).unwrap_or(false);
...
control(
    "L3-registry",
    Maturity::L3,
    "project is registered (L3 registry)",
    at_least(Maturity::L3) && registration_known,
    registered,
    if registration_known {
        vec![if registered {
            "project is registered".to_string()
        } else {
            "project is not registered".to_string()
        }]
    } else {
        vec!["no registry observation was supplied; the registration state is unknown to the doctor".to_string()]
    },
)
```

The control's id, its level, its description and its evidence for the
two known states are unchanged. Only the unknown state is new, and it is
recorded as not applicable — the control's own way of saying the doctor
cannot evaluate it — so it is not counted as an unmet control and the
maturity-requirements finding does not fail the project for it.

This is the same rule `doctor-finding-applicability` established for a
configuration finding and the control that mirrors it: the finding and
its control never disagree about whether the project requires the thing.

#### The L1-structure control

The control's requirement text names the two checks it performs:

```rust
control(
    "L1-structure",
    Maturity::L1,
    "the project carries its manifest and its profile's build definition (L1 structure)",
    at_least(Maturity::L1),
    structure_met,
    structure_evidence,
)
```

The control's id, its level, its applicability, its evidence and the
canonical requirement's list of L1 controls are unchanged. Only the text
stops claiming a comparison that never happens.

**The doctor does not start comparing the layout.** A control that
compared the project with the profile's declared layout would fail four
of the six supported profiles' own scaffolds, because the scaffold is
the deterministic, minimal subset of the profile's declared structure.
The profile's layout stays the profile's declaration, and `forge profile
inspect` keeps showing it. The doctor stops claiming to check it.

#### The dependency-drift evidence

The evidence names what the check compared:

```text
manifest runtime language 'typescript' agrees with profile language 'typescript'
the profile's build definition is present (package.json)
```

and, when the manifest declares no runtime language, it says that no
language was compared:

```text
the manifest declares no runtime language; no manifest language was compared
```

The check is unchanged: the manifest's runtime language against the
profile's language, and the profile's build definition's presence. The
drift evidence itself is unchanged, because it names the two facts the
check compared.

### The claims are pinned

A claim that names a check is a claim that can drift. The contract check
holds the four to what the code reads:

| Claim | The contract check asserts |
|---|---|
| the registry observation | a doctor run without an observation does not claim the project is unregistered, and a run with an observation that says "not registered" still warns |
| the registry callers | the CLI and the MCP observation say "not registered" when the registry holds no record |
| the L3-registry control | it is applicable only when the doctor holds an observation, and its evidence names the unknown |
| the L1-structure control | its requirement text names the manifest and the build definition, and it does not claim a layout comparison |
| the dependency-drift evidence | it names the manifest's runtime language and the profile's language |

## What Is Not Touched, And Why

| Surface | Why it is not in this package |
|---|---|
| The profile's `layout` field | The layout is the profile's declared project structure and stays that. The doctor simply stops claiming to compare a project with it. |
| The drift check's logic | It still compares the manifest's runtime language with the profile's language and asks whether the build definition is present. Only the evidence's wording changes. |
| The staleness rule | An observation that predates the manifest is still stale, and still reported as stale. |
| The release lane | It still runs the doctor without an observation. The doctor now reports what it does not know. |
| The API surfaces | They resolve the project from the registry and always supply a registered observation. |
| The checker plane | It renders the findings; the rendered claims change with them. |
| The doctor's other findings | The manifest, profile, features, build configuration, deployment, repository, CI, documentation, workspace metadata, declaration vocabulary, DriftWatch, policy, gate evidence, release evidence and delivery findings keep their behaviour. |
| The doctor's other controls | Only the `L1-structure` description and the `L3-registry` applicability change; the rest keep their text, their applicability and their evidence. |
| The manifest, the catalogue | No field is added, renamed or defaulted. |

## User Experience and Interface

Before, the doctor claimed four things it did not check:

```text
  [PASS] dependency-drift (automatic)
    evidence: manifest runtime agrees with on-disk dependency evidence

  [PASS] L1-structure (L1): project structure matches the profile layout (L1 structure)
    evidence: forge.yaml and profile build definition are present
```

and the release lane's doctor report claimed a registration state the
doctor never looked at:

```text
  [WARN] registry-observation (manual)
    evidence: project is not registered; maturity evidence is local-only
    detail: project is not registered
```

After, each statement names the check behind it:

```text
  [PASS] dependency-drift (automatic)
    evidence: manifest runtime language 'rust' agrees with profile language 'rust'
    evidence: the profile's build definition is present (Cargo.toml)

  [PASS] L1-structure (L1): the project carries its manifest and its profile's build definition (L1 structure)
    evidence: forge.yaml and profile build definition are present
```

and the registry observation reports the unknown:

```text
  [PASS] registry-observation (automatic)
    evidence: no registry observation was supplied; the registration state is unknown to the doctor
    detail: no registry observation was supplied; the doctor does not open the registry itself
    applicability: not applicable
```

A project that is genuinely unregistered still warns, because the
caller that consulted the registry says so:

```text
  [WARN] registry-observation (automatic)
    evidence: project is not registered; maturity evidence is local-only
    detail: project is not registered
```

## Behavioral Model

| Situation | Outcome |
|---|---|
| The doctor runs with a fresh, registered observation | The `registry-observation` finding passes, as today. |
| The doctor runs with an observation that says the project is not registered | The finding warns, as today. |
| The doctor runs with a stale observation | The finding warns that the observation is stale, as today. |
| The doctor runs with no observation at all | The finding reports the unknown state as not applicable; it never claims the project is unregistered. |
| The CLI or the MCP method runs on a project the registry does not hold | The caller supplies an observation that says so; the finding warns that the project is not registered. |
| The doctor assesses a project at L3 or above with an observation | The `L3-registry` control is applicable and reports the registration state, as today. |
| The doctor assesses a project at L3 or above without an observation | The `L3-registry` control is not applicable and names the unknown; the project is not failed for it. |
| A project's manifest language agrees with its profile | The drift finding passes and names both languages. |
| A project's manifest declares no runtime language | The drift finding passes and says that no language was compared. |
| A project's manifest language disagrees with its profile | The drift finding warns and names both languages, as today. |
| A project's build definition is missing | The drift finding warns with the missing evidence, as today. |
| A project is assessed for structure | The control's text names the manifest and the build definition. |

## Failure and Boundary Policy

- **A fact the doctor cannot know is never stated.** The registration
  state without an observation is the case in point.
- **A known absence still warns.** A project that is genuinely
  unregistered, and a project whose build definition is genuinely
  missing, are still reported as such.
- **The claims are pinned.** The contract check fails when a claim
  names a check the code does not perform.
- **The check is read-only.** Nothing in this package writes to the
  inspected project.
- **The doctor's report stays stable.** The finding IDs, the control
  IDs, the levels and the evidence shape are unchanged.

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

1. `forge doctor` on a project: the drift evidence names both languages
   and the structure control names the two checks it performs.
2. The release lane's doctor run: the registry observation reports the
   unknown state instead of claiming the project is unregistered.
3. `forge doctor` for a project that is genuinely unregistered: the
   warning stays.

## Implementation Plan

**The findings.** `src/doctor/runner.rs`: the `registry-observation`
finding reports the unknown state when no observation is supplied, and the
`dependency-drift` evidence names the manifest's and the profile's
language.

**The controls.** `src/doctor/assessment.rs`: the `L1-structure`
control's requirement text names the checks it performs, and the
`L3-registry` control follows the observation.

**The callers.** `src/cli/projects.rs` and `src/mcp/params.rs`: an
unregistered project supplies an observation that says so.

**The tests.** `tests/doctor_claim_evidence_contract.rs` pins the four
claims; `src/doctor/doctor_tests.rs` gains the case-level checks.

## Decision Ledger

- **Resolved:** with no observation, the `registry-observation` finding
  reports the unknown state as not applicable, in the house's
  not-applicable shape.
- **Resolved:** an observation that says the project is not registered
  still warns, and a stale observation is still stale.
- **Resolved:** the CLI and the MCP method supply an observation that
  says "not registered" when the registry holds no record, so the
  genuine warning is kept and backed by a fact the caller read.
- **Resolved:** the `L3-registry` control is applicable only when the
  doctor holds an observation, and names the unknown when it does not.
- **Resolved:** the `L1-structure` control's text names the manifest and
  the build definition; it stops claiming a layout comparison.
- **Resolved:** the doctor does not start comparing the layout. The
  scaffolds are the deterministic subset of the profile's declared
  structure, so a layout comparison would fail four of the six
  supported profiles' own scaffolds.
- **Resolved:** the `dependency-drift` evidence names the manifest's
  runtime language and the profile's language; the check's logic is
  unchanged.
- **Resolved:** the release lane, the API surfaces and the checker plane
  are untouched; they read the change.
- **Implementation authorized (2026-10-22):** the owner directed the
  trunk repair programme and authorized this package in the approved
  order, after `doctor-finding-applicability` closed the applicability
  gap.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| A finding and a control state only what the doctor read | The report no longer claims what was not checked | The statements name their checks | The four claims | `src/doctor/runner.rs`, `src/doctor/assessment.rs` | The unknown state is reported; a known absence still warns | 3.1, 3.2, 3.4 | The contract test; the real runs |
| The registry observation never guesses | The release's doctor report stops contradicting the registry | The unknown is reported | The three states | `src/doctor/runner.rs` | No observation → not applicable; unregistered → warning | 3.1, 3.3 | The contract test |
| The callers say which situation they are in | The CLI keeps its genuine warning | The warning is backed by a read fact | The unregistered observation | `src/cli/projects.rs`, `src/mcp/params.rs` | No record → the warning stays | 3.3 | The contract test; the real runs |
| The registry control follows the observation | The finding and the control agree | The control names the unknown | The applicability | `src/doctor/assessment.rs` | No observation → not applicable; unregistered → unmet | 3.2 | The contract test |
| The structure control names its checks | The report stops claiming a layout comparison | The control's text names the checks | The description | `src/doctor/assessment.rs` | The control still gates the same way | 3.2 | The contract test |
| The drift evidence names what it compared | The evidence stops crediting unread evidence | The evidence names both languages | The evidence | `src/doctor/runner.rs` | A drift still warns with both languages | 3.1, 3.4 | The contract test |
