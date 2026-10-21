# Change: doctor-build-definition-truth

## Why

`forge doctor` judges a project against the build definition its profile
expects. The doctor's table of those expectations is incomplete:

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

Three of the six supported profiles have a row. `react-web` — a supported
profile with a tested native scaffold, promoted in v0.2 — does not. So a
healthy React project is reported as broken:

```console
$ forge new /tmp/rw/app --profile react-web --id rw-app
$ forge doctor /tmp/rw/app
verdict: not healthy
  [WARN] dependency-drift (automatic)  evidence: unknown profile: no expected build definition
  [FAIL] build-config (automatic)     evidence: unknown profile: no expected build definition
```

The project is not broken. It has `package.json`, which is exactly what the
`react-web` profile's generator writes. The doctor simply does not know
what a React project looks like.

The evidence is worse than the verdict. `unknown profile: no expected build
definition` names the project as the problem, when the gap is in the
doctor's own table. `profile-known` reported `PASS` for the same project in
the same report, so the report contradicts itself: the profile is known, and
the profile is unknown.

The same misleading `FAIL` is reachable for a profile the catalogue does
know but the doctor does not judge — a planned profile id such as
`flutter-client`. `inspect_profile` finds planned descriptors, so
`profile-known` passes, and the doctor's table has no row for it. The doctor
blames the project for a profile it has not learned.

And nothing stops the gap from growing. The doctor reads the profile id
from the manifest and looks it up in a hand-written table; the catalogue
lists the supported profiles in a different place. A new supported profile
can be added to the catalogue, generation and preflight without the
doctor's table ever being updated, and the result is the same false
failure.

## What Changes

- **The doctor's build-definition table covers every supported profile.**
  `react-web` joins the table with `package.json`. The table becomes one
  explicit, typed list: `aspnet-web` expects a `*.csproj`, `flutter-app`
  expects `pubspec.yaml`, `nextjs-web` and `react-web` expect
  `package.json`, `python-service` expects one of `pyproject.toml`,
  `requirements.txt`, `setup.py` or `setup.cfg`, and `rust-web` expects
  `Cargo.toml`.
- **A profile the doctor cannot judge is `UNAVAILABLE`, not `FAIL`.** When
  the doctor has no expected build definition for the profile, the
  `build-config` and `dependency-drift` findings are reported as
  `UNAVAILABLE` — a required inspector that cannot run — and the evidence
  names the profile. The doctor never reports its own gap as a defect of
  the project it inspects.
- **The table is pinned against the catalogue.** A repository contract check
  reads the supported catalogue and the doctor's table and fails when they
  disagree, naming the profile. A supported profile cannot be added
  without the doctor learning it.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The doctor's build-definition probes | `forge` | Yes | `src/doctor/probes.rs`: the table, the expectation type and the outcome type. |
| The doctor's finding inventory | `forge` | Yes | `src/doctor/runner.rs`: `build-config` and `dependency-drift` carry the three states. |
| The doctor's maturity assessment | `forge` | Yes | `src/doctor/assessment.rs`: the L1 structure and build controls read the new outcome. |
| The profile catalogue | `forge` | Read only | `src/profile/catalog.rs` is unchanged; the catalogue is the authority the table is pinned against. |
| The doctor contract test | `forge` | Yes | `tests/doctor_contract.rs` keeps its assertions. |
| The build-definition contract test | `forge` | Yes | `tests/doctor_build_definition_contract.rs`: the pinned table and the healthy-React behaviour. |
| The generator | `forge` | Read only | `src/generate/engine.rs` already writes the file each row expects. |
| The build lane, the readiness matrix | `forge` | Read only | They run the profile's declared build command; their contracts do not change. |
| The manifest | `forge` | Read only | No field is added, renamed or defaulted. |
| The `doctor-maturity-assessment` capability | `forge` | Modified | The finding inventory states how a build definition is judged. |
| The `profile-registry` capability | `forge` | Read only | Its requirements already hold; no wording moves. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md:962` (§23 Doctor) lists build configuration as an inspection; `requirement.md:1065` (§25 L1) requires a "Docker or equivalent build definition"; §9 and §14 make the profile the unit of generation. |
| Contracts | The doctor's finding inventory keeps its IDs, statuses and evidence shape. The `build-config` finding gains one state (`unavailable`) that the inventory already allows. |
| Callers | `forge doctor` (CLI), `POST /projects/{id}/doctor` (API) and the maturity assessment all read the same findings. |
| Persistence | Nothing is persisted. The doctor's table is compiled in. |
| Integrations | None. No new toolchain, network or credential surface. |
| Failures | A doctor gap becomes `UNAVAILABLE` — a status the doctor's contract already defines for an inspector that cannot run. No new error code. |
| Tests | A new `tests/doctor_build_definition_contract.rs` pins the table and the behaviour; the existing doctor tests keep passing unchanged. |
| Dependencies | None added. |
| Concerns | Quality and compatibility: the table covers every supported profile and the check fails when it does not. Security: no new file, process, network or credential surface; the doctor stays read-only. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `doctor-build-definition-truth` | forge | Rust | added | this package |
| `doctor-maturity-assessment` | forge | Rust | modified | the finding inventory states how a build definition is judged and what an unjudgeable profile reports |

## Non-Goals

- **The doctor's other probes are not touched.** Dependency drift still
  compares the manifest's runtime language with the profile's language.
  Deployment, CI, documentation, repository, DriftWatch and registry
  observation probes keep their behaviour.
- **The planned profiles are not promoted.** `flutter-client` stays
  planned, and the doctor still does not judge a planned profile's build
  definition — it reports `UNAVAILABLE` instead of a false failure.
- **The doctor does not read the generator's output.** The expectation is
  a declared table, not a probe of what a scaffold would write.
- **The build lane's contract does not change.** It still runs the
  profile's declared build command and reports it.
- **The manifest does not carry the build definition.** The file a profile
  expects is the doctor's expectation; it is not a new manifest field and
  not a descriptor field.
- **The doctor does not judge a build by running it.** The doctor's
  finding inventory is read-only and stays read-only.
