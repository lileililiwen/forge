# Change: doctor-claim-evidence

## Why

`forge doctor` reports what it finds. Four of its statements are about
things the doctor never looked at.

**The registry-observation finding claims a fact the doctor cannot know.**

```rust
// src/doctor/runner.rs, before this change
match observation {
    Some(obs) if obs.registered => { /* fresh or stale */ }
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

The doctor never opens the registry. The registration state is known only
when the caller supplies a read-only observation. The `_` arm covers two
different situations and states the same thing about both:

| Situation | What the doctor knows | What it says |
|---|---|---|
| The caller supplied an observation that says the project is not registered | the project is not registered | correct |
| The caller supplied no observation at all | nothing | "project is not registered" |

The release lane is the second caller: it resolved the project *from* the
registry and then ran the doctor without the observation, so its own doctor
report says the project is not registered.

The doctor's own module contract states the rule the code breaks:

> Registry access, when the caller supplies it, is limited to a read-only
> observation used to mark stale results as stale.

**The callers cannot tell the two situations apart.**

```rust
// src/cli/projects.rs and src/mcp/params.rs, before this change
fn observation_for(registry: &Registry, path: &Path) -> Option<RegistryObservation> {
    let canonical = path.canonicalize().ok()?.display().to_string();
    let record = registry
        .list()
        .ok()?
        .into_iter()
        .find(|p| p.path == canonical)?;   // <- no record: None
    ...
}
```

A project that is not in the registry and a registry that was never
consulted both arrive as `None`. So the CLI cannot say "this project is
not registered" — it can only say nothing at all. Fixing the doctor
without fixing the callers would replace a false claim with a false
silence, and the CLI's genuine "not registered" warning would be lost.

**The L3-registry control states the same fact the doctor cannot know.**

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

With no observation the control reports "project is not registered" and
counts as an unmet L3 control, so a project is failed for a fact the
doctor never read. Fixing the finding while leaving the control would
make one report contradict itself.

**The L1-structure control claims it compared the project with the
profile's layout.**

```text
  [PASS] L1-structure (L1): project structure matches the profile layout (L1 structure)
    evidence: forge.yaml and profile build definition are present
```

The control checks two things: that the manifest is present and that the
profile's build definition is present. The profile's layout is never
read, by the doctor or by any other surface. And the claim is false for
four of the six supported profiles: `rust-web` declares `tests/`,
`react-web` declares `public/` and `tests/`, `flutter-app` declares
`android/` and `ios/`, and `aspnet-web` declares `src/` and `tests/` —
and Forge's own scaffolds write none of them. So the doctor tells the
operator that a generated project's structure matches its profile's
declared structure, while a directory listing of that project shows it
does not.

**The dependency-drift finding credits on-disk evidence it never read.**

```text
  [PASS] dependency-drift (automatic)
    evidence: manifest runtime agrees with on-disk dependency evidence
```

The check compares the manifest's runtime language with the *profile's*
language, and asks whether the profile's build definition file is
present. No language marker is read from the project. The evidence names
on-disk dependency evidence the check never consulted.

## What Changes

- **The doctor never states a fact it cannot know.** The
  `registry-observation` finding keeps three states: a fresh observation,
  a stale observation, and an observation that says the project is not
  registered. With no observation at all, the finding reports the unknown
  state — in the house's own not-applicable shape — instead of claiming
  the project is not registered.
- **The callers say which situation they are in.** A caller that
  consulted the registry and found no record for the project supplies an
  observation that says so, so the CLI and the MCP method keep the
  genuine "not registered" warning. A caller that never consulted the
  registry supplies no observation, so the doctor reports the unknown.
- **The L3-registry control follows the same observation.** It is
  applicable only when the doctor holds an observation, and its evidence
  names the unknown when it does not, so the finding and the control
  never contradict each other.
- **The L1-structure control names the check it performs.** Its
  requirement text names the manifest and the profile's build definition,
  the two things it actually checks, instead of claiming a comparison
  with the profile's layout that never happens.
- **The dependency-drift finding names what it compared.** Its evidence
  names the manifest's runtime language and the profile's language, and
  the build definition, instead of crediting on-disk dependency evidence
  the check did not read.
- **The claims are pinned.** A repository contract check holds the
  doctor's vocabulary to what the code reads, so a claim cannot drift
  back into naming a check that is not performed.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The doctor's finding inventory | `forge` | Yes | `src/doctor/runner.rs`: the `registry-observation` finding and the `dependency-drift` evidence. |
| The doctor's maturity controls | `forge` | Yes | `src/doctor/assessment.rs`: the `L1-structure` requirement text and the `L3-registry` applicability. |
| The CLI's registry observation | `forge` | Yes | `src/cli/projects.rs`: an unregistered project supplies an observation that says so. |
| The MCP method's registry observation | `forge` | Yes | `src/mcp/params.rs`: the same. |
| The doctor's contract test | `forge` | Read only | `tests/doctor_contract.rs` keeps its assertions. |
| The claim-evidence contract test | `forge` | Yes | `tests/doctor_claim_evidence_contract.rs`: the four claims against what the doctor reads. |
| The doctor unit tests | `forge` | Yes | `src/doctor/doctor_tests.rs` gains the case-level checks. |
| The checker plane | `forge` | Read only | It renders the findings; the claims it renders change with them. |
| The release lane | `forge` | Read only | It runs the doctor without an observation; the doctor now says what it does not know instead of claiming the project is unregistered. |
| The API surfaces | `forge` | Read only | They resolve the project from the registry and always supply a registered observation. |
| The profile catalogue | `forge` | Read only | The `layout` field is unchanged; the doctor simply stops claiming to compare it. |
| The manifest | `forge` | Read only | No field is added, renamed or defaulted. |
| The `doctor-claim-evidence` capability | `forge` | Added | This package. |
| The `doctor-maturity-assessment` capability | `forge` | Modified | The inventory states that a finding and a control state only what the doctor read. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md:962` (§23 Doctor) asks the doctor to inspect the project with evidence; `requirement.md:1046` (§25 Maturity Model) asks for evidence-based levels. |
| Contracts | The finding IDs, the control IDs, the status set and the evidence shape are unchanged. Three texts and two applicability rules change. |
| Callers | `forge doctor` (CLI), the MCP doctor method, the API route, the checker document and the release lane read the same report. |
| Persistence | Nothing is persisted. |
| Integrations | None. No new toolchain, network or credential surface. The doctor stays read-only. |
| Failures | No new error code. A project that is genuinely unregistered still warns, on the CLI and the MCP method. |
| Tests | A new `tests/doctor_claim_evidence_contract.rs` pins the four claims; the existing doctor tests keep passing. |
| Dependencies | None added. |
| Concerns | Quality and compatibility: the doctor's statements now match the checks it performs, and the contract check holds them to it. Security: no new file, process, network or credential surface. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `doctor-claim-evidence` | forge | Rust | added | this package |
| `doctor-maturity-assessment` | forge | Rust | modified | the inventory states that a finding and a control state only what the doctor read |

## Non-Goals

- **The profile's `layout` is not changed.** The layout stays the
  profile's declared project structure and `forge profile inspect` still
  shows it. The doctor simply stops claiming to compare a project with
  it.
- **The doctor does not start checking the layout.** A control that
  compared the layout would fail four of the six supported profiles'
  own scaffolds, because the scaffolds are the deterministic, minimal
  subset of the profile's declared structure. The change makes the
  control honest about what it checks instead.
- **The drift check does not start reading language markers.** The check
  still compares the manifest's runtime language with the profile's
  language and asks whether the build definition is present. Its evidence
  names that.
- **The staleness rule is untouched.** An observation that predates the
  manifest is still reported as stale.
- **The release lane is untouched.** It still runs the doctor without an
  observation; the doctor now reports what it does not know.
- **The doctor's other findings and controls are untouched.**
- **The doctor stays read-only.**
