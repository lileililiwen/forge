# Change: lifecycle-observation-closure

## Why

`forge doctor` is the operator's read-only question: *is this project in good
shape?* It answers with evidence about the project directory — the manifest,
the profile, the dependency drift, the build and deployment configuration, the
repository, the CI, the documentation, the maturity controls.

It never looks at what the delivery lanes recorded. The
project-to-production workflow writes every delivery verb into the
`operations` journal (`delivery.preflight`, `delivery.stage`,
`delivery.promote`, `delivery.hermora`) and projects them into a typed phase.
That projection is "the single door every transport enters" when answering
`forge delivery status <project>` or `GET /v1/projects/{id}/delivery`.

The doctor does not enter it. So a project whose last delivery **failed** is
reported healthy, and a project that was never delivered at all is reported
healthy. The spine the programme repairs — adopt → build → verify → release →
deliver → observe — has a hole in it exactly where the delivery lands: the
surface that judges the project never sees the delivery.

The fix is not a new probe and not a second vocabulary. The delivery state
already exists, it already has one door, and it already has one owner. The
doctor has to read it.

## What Changes

- **The doctor reads the delivery state.** `RegistryObservation` carries what
  the caller could observe about the project's delivery, and the doctor turns
  it into a `delivery-state` finding: the phase, the environment, the
  revision the delivery transferred and the time it last changed.
- **The finding is typed.** A failed delivery is `FAIL`, a delivery in
  progress is `WARN`, a healthy delivery is `PASS`, and a project with no
  recorded delivery is reported as not applicable — never as an alarm. A
  project that is not registered gets no delivery finding at all; the doctor
  already reports the registration state.
- **One door, every surface.** The CLI (`forge doctor`, `forge check`), the
  API (`POST /projects/{id}/doctor`, the project status document) and the
  external checker all read the same delivery state through the same
  function, so the finding is the same everywhere.
- **Still read-only.** The doctor reads the journal and writes nothing: no
  journal row, no registry write, no target write, no provider invocation.
  The delivery lane's own verbs are not triggered by observing.

## Package Boundary

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `lifecycle-observation-closure` | forge | Rust | implemented | this package |
| `doctor-maturity-assessment` | forge | Rust | modified | the finding inventory gains the delivery state |
| `project-to-production-workflow` | forge | Rust | read only | the projection and its phase vocabulary are read, never changed |
| `external-checker-emission` | forge | Rust | read only | the checker document gains the finding through the doctor report it already carries |
| The delivery verbs (`preflight`, `stage`, `promote`, `hermora-retry`) | forge | Rust | read only | observing a delivery never runs one |
| The manifest, the profile catalog, the publish lane, the deploy lane | forge | Rust | read only | not edited |

## Non-Goals

- **The doctor never writes.** No journal row, no registry write, no
  provider call, no target write. Observation stays observation.
- **The delivery projection is not changed.** The doctor reads the phase the
  lane publishes. If the projection is wrong, that is a delivery-lane change,
  not a doctor change.
- **The doctor does not decide whether a delivery is required.** A project
  that declares no delivery is reported as not applicable. What a project
  must deliver is the manifest's business.
- **The doctor's other findings are untouched.** The inventory grows by one
  entry; nothing else changes.
