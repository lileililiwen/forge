## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The doctor's build-definition probes | `forge` | Yes. `src/doctor/probes.rs`. |
| The doctor's finding inventory | `forge` | Yes. `src/doctor/runner.rs`. |
| The doctor's maturity assessment | `forge` | Yes. `src/doctor/assessment.rs`. |
| The build-definition contract test | `forge` | Yes. `tests/doctor_build_definition_contract.rs`. |
| The profile catalogue | `forge` | Read only. It is the authority the table is pinned against. |
| The generator | `forge` | Read only. It already writes the file each row expects. |
| The build lane, the readiness matrix, the manifest | `forge` | Read only. |

## The Gap

**The doctor's table is missing a supported profile.**

```rust
// src/doctor/probes.rs, before this change
fn expected_build_file(profile: &str) -> Option<&'static str> {
    match profile {
        "rust-web" => Some("Cargo.toml"),
        "nextjs-web" => Some("package.json"),
        "flutter-app" => Some("pubspec.yaml"),
        "aspnet-web" => None,     // any *.csproj
        "python-service" => None, // pyproject.toml or requirements.txt
        _ => None,
    }
}
```

`react-web` is a supported profile with a tested native scaffold. It has no
row. The generator writes `package.json` for it
(`src/generate/engine.rs:187`), and `nextjs-web` — the same ecosystem, the
same file — does have a row.

**So a healthy project is reported as broken.**

```console
$ forge new /tmp/rw/app --profile react-web --id rw-app
$ forge doctor /tmp/rw/app
verdict: not healthy
  [WARN] dependency-drift (automatic)  evidence: unknown profile: no expected build definition
  [FAIL] build-config (automatic)     evidence: unknown profile: no expected build definition
```

The project has `package.json`. The doctor cannot name it, so it says
"unknown profile: no expected build definition" and fails the project.

**The report contradicts itself.** `profile-known` is computed from
`inspect_profile(...).is_ok()` (`src/doctor/runner.rs:45`), and
`react-web` is in the supported catalogue, so the same report carries:

```text
[PASS] profile-known  evidence: profile 'react-web' matches a versioned MVP descriptor
[FAIL] build-config  evidence: unknown profile: no expected build definition
```

The profile is known and unknown in the same report. The evidence names the
project as the cause of a gap that lives in the doctor.

**The gap is reachable for a profile the catalogue knows.** `inspect_profile`
searches the supported catalogue and then the planned one
(`src/profile/resolve.rs:27`). A project whose manifest names a planned
profile such as `flutter-client` passes `profile-known` and then hits the
same missing row, with the same false failure.

**Nothing keeps the table in step with the catalogue.** The catalogue lists
the supported profiles in `src/profile/catalog.rs`; the doctor lists them in
`src/doctor/probes.rs`. They are two hand-written lists, and only one of
them is checked.

## The Decision

### The table is complete and typed

The expectations become one typed list, one row per supported profile:

```rust
/// What a project must contain for the doctor to judge its build
/// configuration present.
pub enum BuildDefinition {
    /// A file with this exact name in the project root.
    File(&'static str),
    /// Any file in the project root with this extension.
    Extension(&'static str),
    /// Any one of these files in the project root.
    OneOf(&'static [&'static str]),
}

/// The expected build definition of every profile the doctor judges.
pub fn expected_build_definition(profile: &str) -> Option<BuildDefinition> {
    match profile {
        "aspnet-web" => Some(BuildDefinition::Extension("csproj")),
        "flutter-app" => Some(BuildDefinition::File("pubspec.yaml")),
        "nextjs-web" => Some(BuildDefinition::File("package.json")),
        "python-service" => Some(BuildDefinition::OneOf(&[
            "pyproject.toml",
            "requirements.txt",
            "setup.py",
            "setup.cfg",
        ])),
        "react-web" => Some(BuildDefinition::File("package.json")),
        "rust-web" => Some(BuildDefinition::File("Cargo.toml")),
        _ => None,
    }
}
```

The three shapes are the three shapes the expectations actually have:
`aspnet-web` writes `{id}.csproj`, so its name is not known in advance and
the expectation is the extension; `python-service` accepts any of four
manifests; every other profile expects one named file.

### The outcome has three states, not two

The old probe answered a yes/no question with a tuple. The doctor needs to
say three things:

```rust
pub enum BuildDefinitionOutcome {
    /// The definition is present; the evidence names what was found.
    Present(Vec<String>),
    /// The definition is missing; the evidence names what was expected.
    Missing(Vec<String>),
    /// The doctor has no expected build definition for this profile.
    UnknownProfile,
}
```

`Present` and `Missing` are the two states the doctor already had. The
third is the one it lacked: *the doctor cannot judge this profile*. It was
folded into `Missing`, and `Missing` became a failure of the project.

### A doctor gap is `UNAVAILABLE`, never a `FAIL`

The doctor's contract already has the state this needs:

> Unknown or unexecuted checks are reported explicitly and never converted
> to `PASS`.

and the opposite rule is the same rule read from the other side: a check
the doctor cannot run must not be converted into a `FAIL` of the project
either. So:

| Profile | Build definition | `build-config` finding |
|---|---|---|
| Known to the doctor | present | `PASS` |
| Known to the doctor | missing | `FAIL`, naming the expected file |
| Not known to the doctor | — | `UNAVAILABLE`, naming the profile |

`dependency-drift` follows the same rule: the build definition is one of
its two inputs, so a profile the doctor cannot judge leaves the drift
unevaluated, and the finding is `UNAVAILABLE` rather than `WARN` with the
evidence "unknown profile".

The evidence for the `UNAVAILABLE` state names the profile and says what
is missing, so the operator sees a doctor gap and not a broken project:

```text
the doctor has no expected build definition for profile 'flutter-client'
```

### The table is pinned against the catalogue

A complete table is still two hand-written lists that can drift apart again.
The contract check closes that:

```rust
/// `(profile, expected build definition)` for every supported profile.
const MATRIX: [(&str, &[&str]); 6] = [
    ("aspnet-web", &["*.csproj"]),
    ("flutter-app", &["pubspec.yaml"]),
    ("nextjs-web", &["package.json"]),
    ("python-service", &["pyproject.toml", "requirements.txt", "setup.py", "setup.cfg"]),
    ("react-web", &["package.json"]),
    ("rust-web", &["Cargo.toml"]),
];
```

The check reads the supported catalogue (`forge::profile::list_profiles()`)
and the doctor's table (`forge::doctor::expected_build_definition`) and
asserts three things:

| Check | Fails when |
|---|---|
| Every supported profile has an expectation | A profile is added to the catalogue and the doctor does not learn it. |
| Every expectation belongs to a supported profile | The doctor keeps a row for a profile that no longer exists. |
| Every expectation matches the pinned table | A row's expectation changes without the table being updated in the same change. |

This is the house pattern: `tests/workspace_metadata_contract.rs` pins the
governance profile and the native test command per profile in exactly this
shape, and `tests/profile_build_command_truth_contract.rs` pins the
catalogue's build commands the same way.

### The behaviour is checked through the doctor, not only through the table

A table can be right while the doctor still reports the wrong thing, so the
contract test also drives the doctor:

| Project | Expected |
|---|---|
| `react-web` with `package.json` | `build-config` is `PASS`, `dependency-drift` is `PASS` |
| `react-web` without `package.json` | `build-config` is `FAIL`, evidence names `package.json` |
| a profile the doctor does not judge | `build-config` is `UNAVAILABLE`, evidence names the profile |

## What Is Not Touched, And Why

| Surface | Why it is not in this package |
|---|---|
| The profile catalogue (`src/profile/catalog.rs`) | The catalogue is right; the doctor's copy of it was not. The catalogue is the authority the table is pinned against, so it stays the single source. |
| The generator (`src/generate/engine.rs`) | It already writes the file each row expects: `Cargo.toml`, `package.json`, `package.json`, `pyproject.toml`, `*.csproj`, `pubspec.yaml`. |
| The other doctor probes | Dependency language drift, deployment, CI, documentation, repository, DriftWatch, registry observation and release evidence keep their behaviour. |
| The manifest | No field is added, renamed or defaulted. The file a profile expects is the doctor's expectation, not a manifest fact. |
| The descriptor | No field is added. The build *command* is the descriptor's business; the build *definition file* is the doctor's. |
| The build lane (`src/build/mod.rs`) | It runs the profile's declared build command and reports it. That contract is already right. |
| The planned profiles | `flutter-client` stays planned and stays unjudged by the doctor. The change turns its misleading `FAIL` into an honest `UNAVAILABLE`. |
| The doctor's read-only promise | Nothing new is read and nothing is written. The doctor still reads the project directory and never modifies it. |

## User Experience and Interface

Before, a healthy React project failed the doctor:

```console
$ forge doctor /tmp/rw/app
verdict: not healthy
  [WARN] dependency-drift (automatic)  evidence: unknown profile: no expected build definition
  [FAIL] build-config (automatic)     evidence: unknown profile: no expected build definition
```

After, it passes the build configuration and the drift check:

```console
$ forge doctor /tmp/rw/app
  [PASS] build-config (automatic)  evidence: package.json
  [PASS] dependency-drift (automatic)  evidence: manifest runtime agrees with on-disk dependency evidence
```

A React project that really has no `package.json` still fails, and the
evidence names the file the profile expects:

```console
  [FAIL] build-config (automatic)  evidence: missing build definition 'package.json'
```

And a profile the doctor does not judge is no longer blamed on the
project:

```console
  [UNAVAILABLE] build-config (automatic)  evidence: the doctor has no expected build definition for profile 'flutter-client'
```

## Behavioral Model

| Situation | Outcome |
|---|---|
| A supported profile with a complete table | The project is judged against that row: `PASS` when the file is there, `FAIL` when it is not. |
| A supported profile whose row is missing from the doctor's table | The `build-config` finding is `UNAVAILABLE` and names the profile. The repository's contract check fails and names the profile too. |
| A profile the catalogue does not know | Unchanged: `profile-known` is `FAIL` and the descriptor-gated checks are `UNAVAILABLE`. |
| A planned profile | `profile-known` is `PASS`; the build expectation is `UNAVAILABLE`, because the doctor does not judge a planned profile's build definition. |
| A project whose file exists but is empty | Unchanged: the doctor checks presence, not content. |
| `aspnet-web` with a `*.csproj` in a subdirectory | Unchanged: the expectation is a file in the project root. |
| `python-service` with any of the four accepted manifests | `PASS`, and the evidence names the file that was found. |

## Failure and Boundary Policy

- **A doctor gap is `UNAVAILABLE`.** It is a status the doctor's contract
  already defines, and it never hides behind a `PASS`.
- **The `UNAVAILABLE` evidence names the profile.** A status without a
  name is not actionable.
- **A missing build definition is still a `FAIL`.** The fix must not soften
  the check into a warning; a React project with no `package.json` is a
  broken project.
- **The evidence names the expected file.** `missing build definition
  'package.json'` says what to create.
- **The check is read-only.** Nothing in this package writes to the
  inspected project.
- **The pinned table is the friction.** Changing an expectation fails the
  contract check until the table is updated in the same change. That is
  the intended friction.

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

1. `forge new --profile react-web` then `forge doctor` on the generated
   project: the `build-config` finding is `PASS`, and the verdict no longer
   says "not healthy" because of the build definition.
2. The same project with `package.json` removed: `build-config` is `FAIL`
   and the evidence names `package.json`.
3. A manifest that names a profile the doctor does not judge:
   `build-config` is `UNAVAILABLE`, not `FAIL`.

## Implementation Plan

**The table and the outcome.** `src/doctor/probes.rs` gains
`BuildDefinition`, `BuildDefinitionOutcome`,
`expected_build_definition` and a rewritten `has_build_definition` that
returns the three states. `BuildDefinition::expected_names` renders the
expectation in a stable order for evidence and for the pinned table.

**The findings.** `src/doctor/runner.rs`: the `build-config` and
`dependency-drift` findings map the three states onto `PASS` / `FAIL` /
`UNAVAILABLE`.

**The assessment.** `src/doctor/assessment.rs`: the L1 structure and build
controls read the new outcome, and an unjudgeable profile is recorded as
unmet with the profile named.

**The tests.** `tests/doctor_build_definition_contract.rs` pins the table
against the catalogue and drives the doctor through the binary.
`src/doctor/doctor_tests.rs` gains the unit-level check for the three
states.

## Decision Ledger

- **Resolved:** the table is one typed list with one row per supported
  profile, and `react-web` expects `package.json`.
- **Resolved:** the expectation keeps three shapes — a named file, an
  extension, or one of several files — because that is what the profiles
  actually need.
- **Resolved:** a profile the doctor does not judge is `UNAVAILABLE`, not
  `FAIL`. The doctor's gap is never the project's fault.
- **Resolved:** the table is pinned by a repository contract check against
  the supported catalogue, in the house's `MATRIX` shape.
- **Resolved:** the catalogue's `layout` is not the source of the
  expectation. A layout lists a project's directories and files, not its
  build definition: `aspnet-web`'s layout carries no `.csproj` at all (the
  file is named after the project), and `python-service` accepts four
  manifests where its layout names one. The table is an explicit
  declaration, pinned by the contract check.
- **Resolved:** the doctor's other probes, the manifest, the descriptor,
  the generator and the build lane are untouched.
- **Resolved:** the planned profiles are not promoted and are not judged.
- **Implementation authorized (2026-10-21):** the owner directed the trunk
  repair programme and authorized this package in the approved order, after
  `profile-build-command-truth` closed the build-command gap the build lane
  shared.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| The doctor's build definition covers every supported profile | The false `FAIL` on a healthy React project is gone | The doctor passes a healthy project | One typed table with a row per supported profile | `src/doctor/probes.rs` | A supported profile is known; a missing file still fails | 3.1, 3.5 | The contract test; the real run |
| A profile the doctor cannot judge is unavailable, not failed | The doctor stops blaming the project for its own gap | `UNAVAILABLE` names the profile | The three-state outcome | `src/doctor/probes.rs`, `src/doctor/runner.rs`, `src/doctor/assessment.rs` | An unjudgeable profile is `UNAVAILABLE` | 3.2, 3.3 | The contract test |
| The table is pinned against the catalogue | A new supported profile cannot be added without the doctor learning it | The check fails and names the profile | The pinned `MATRIX` | `tests/doctor_build_definition_contract.rs` | The table matches the catalogue in both directions | 3.5 | The contract test |
| The finding inventory states how a build definition is judged | The contract names the three states | The report is consistent | `doctor-maturity-assessment` gains a scenario | `src/doctor/runner.rs` | Present is `PASS`, missing is `FAIL` | 3.2 | The contract test |
