## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The doctor's observation input and its finding | `forge` | Yes. `src/doctor/`. |
| The doctor CLI, the checker CLI and the API doctor surfaces | `forge` | Yes. `src/cli/projects.rs`, `src/api/handlers_changes.rs`, `src/api/status.rs`. |
| The delivery projection and its phase vocabulary | `forge` | No. Read only. |
| The delivery verbs and their journal writes | `forge` | No. Read only. |
| The manifest, the profile catalog, the publish lane, the deploy lane | `forge` | No. Read only. |

## The Hole

The spine the programme repairs is adopt → build → verify → release →
deliver → observe. The doctor is the surface that answers "is this project in
good shape?", and it answers from the project directory alone:

| Finding | Reads |
|---|---|
| `manifest-*` | the manifest and the profile descriptor |
| `dependency-drift` | the dependency files |
| `git-remote` | the repository's remotes |
| `ci-config` | the CI configuration |
| `driftwatch` | the DriftWatch markers |
| `gate-evidence` | the Gate evidence directory |
| `release-evidence` | the consumed release-evidence export |
| `declaration-vocabulary` | the declared vocabulary against the shipped one |
| `registry-observation` | the registry record's freshness |
| `maturity-requirements` | the L0–L4 controls |

Not one of them asks what the delivery lanes recorded. The delivery lane
writes a closed set of verbs into the `operations` journal and projects them
into a closed phase vocabulary; `forge delivery status <project>` reads that
projection, and the API reads the same one. The doctor does not.

The consequence is concrete: a project whose last promotion failed is
reported healthy. The operator's read-only question has a hole exactly where
the delivery lands.

## The Observation

The doctor is a pure function over a project directory plus one
caller-supplied observation. It does not open the registry itself — the
caller does, read-only, and hands it what it read. The observation is the
seam, and it is already there:

```rust
pub struct RegistryObservation {
    pub registered: bool,
    pub observed_at: Option<String>,
    /// What the caller could observe about the project's delivery.
    pub delivery: Option<DeliveryObservation>,
}
```

`Option` on the outer field means "the caller did not look" — the release
lane's own doctor check has no registry handle and passes `None`. The inner
enum carries the three things a reader can actually find:

```rust
pub enum DeliveryObservation {
    /// The delivery lane recorded a phase for the project.
    Recorded {
        phase: DeliveryPhase,
        environment: Option<DeliveryEnvironment>,
        revision: Option<String>,
        updated_at: String,
    },
    /// The project has no recorded delivery.
    Absent,
    /// The delivery state could not be read.
    Unreadable { reason: String },
}
```

The phase type is the delivery lane's own `DeliveryPhase`, not a string. The
doctor never invents a phase and never re-parses a label: the vocabulary has
one owner, and the doctor reads it.

## The Rule

One function reads the delivery state, and every caller uses it:

```rust
// src/doctor/observation.rs
pub fn delivery_observation(
    registry: &Registry,
    project_id: &str,
    now: DateTime<Utc>,
) -> DeliveryObservation
```

It calls the delivery lane's own door — `build_delivery_report` — and maps
the result:

| What the projection says | The observation |
|---|---|
| The project has no delivery rows (phase `draft`) | `Absent` |
| The projection was read | `Recorded { phase, environment, revision, updated_at }` |
| The projection could not be read | `Unreadable { reason }` |

The doctor then judges the phase with a closed rule:

| Phase | Finding | Why |
|---|---|---|
| `healthy`, `hermora-connected` | `PASS` | the delivery reached production and the provider reported success |
| `stage-failed`, `degraded` | `FAIL` | the delivery failed |
| `hermora-failed` | `WARN` | the delivery lane's own wording is "deployment stays healthy"; only the optional enrolment did not complete |
| every other phase (`draft`, `awaiting-stage-confirmation`, `awaiting-production-approval`, `staging`, `stage-healthy`, `production`) | `WARN` | the delivery is in progress, or has not started |

`Absent` is reported as **not applicable** — the doctor's existing convention
for a check that does not apply (the same one `gate-evidence` and
`declaration-vocabulary` use). A project nobody asked to deliver is not
scolded for having no delivery. `Unreadable` is reported as `UNAVAILABLE`,
which the doctor's own contract already treats as "not healthy": a check that
could not run never passes silently.

`FAIL` and `UNAVAILABLE` deny health. `WARN` does not. A delivery in progress
is a fact, not a fault.

## Where The Finding Comes From

`src/doctor/delivery.rs` owns the probe and the rule:

```rust
pub(super) fn delivery_finding(delivery: Option<&DeliveryObservation>) -> Option<Finding>
```

The runner calls it with the observation's delivery field. No observation, no
finding — the caller did not look, and the doctor does not guess.

The evidence names the phase, the environment, the revision and the time the
state last changed:

```
  [FAIL] delivery-state (manual)
    evidence: delivery phase=stage-failed
    evidence: environment=stage
    evidence: delivered revision=0123456789abcdef0123456789abcdef01234567
    evidence: delivery state last changed 2026-10-20T18:11:57Z
    detail: the last delivery failed; run forge delivery status <project> for the evidence
```

Every string passes `policy::redact_credentials` before it reaches the
report, exactly like the doctor's other probes.

## User Experience and Interface

Nothing new to type. `forge doctor` gains one line in the findings table and
one block of evidence. The JSON report gains one finding. The external
checker's document gains an alert when the delivery failed, because the
checker already maps every doctor finding into an alert.

The doctor's verdict changes only when the delivery failed: a project whose
last delivery failed was healthy before and is not healthy now. That is the
whole point.

## Behavioral Model

| Situation | Outcome |
|---|---|
| The project is registered and its last delivery is healthy | `delivery-state` is `PASS`, naming the phase and the revision. |
| The project is registered and its last delivery failed | `delivery-state` is `FAIL`, and the doctor is not healthy. |
| The project is registered and its delivery is in progress | `delivery-state` is `WARN`; the doctor stays healthy. |
| The project is registered and has no delivery | `delivery-state` is reported as not applicable. |
| The project is not registered | No delivery finding. The doctor already reports `registry-observation` as a warning. |
| The delivery state cannot be read | `delivery-state` is `UNAVAILABLE` and names the reason. The doctor is not healthy. |
| The caller did not look at the delivery | No delivery finding. |

## Contract and Compatibility

The doctor report is Forge's own output. It gains one finding, `delivery-state`,
and the finding inventory grows by one entry. No existing finding changes
status, no existing evidence changes, and the report's shape is unchanged
apart from the new entry.

The delivery projection is not touched: `forge-delivery-status/0.1.0` keeps
its contract, its phase vocabulary and its shape. The doctor reads it.

The `RegistryObservation` input to `run_doctor` is a Rust type, not a
contract. It gains one field, so every construction site in the tree is
updated — the CLI, the checker CLI, the API doctor handler, the API status
surface and the doctor's own tests. A caller that does not look at the
delivery passes `None` and sees no delivery finding.

## Failure and Boundary Policy

- **The doctor never writes.** Reading the delivery state opens the registry
  read-only and selects from the `operations` table. No journal row, no
  registry write, no target write, no provider invocation, no adapter.
- **A failed read is visible.** The delivery state that cannot be read is
  reported as `UNAVAILABLE` with the reason. It is never reported as a pass.
- **An unknown project is not a delivery failure.** The observation is only
  built for a project the registry knows; a project that is not registered
  gets no delivery finding.
- **The doctor does not start a delivery.** `forge doctor` never runs a
  preflight, a stage, a promotion or an enrolment. Observation is not action.
- **No secret reaches the report.** The phase, the environment and the
  revision are not credential-shaped, but every rendered string still passes
  `policy::redact_credentials`, so a journal detail that carries one is
  scrubbed before it is shown.

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

A real run: a fixture project registered in a real registry, with delivery
journal rows written by the delivery lane, doctored for real. The report
must name the phase, and the verdict must change with the phase.

## Implementation Plan

**The observation.** `src/doctor/model.rs` gains `DeliveryObservation` and
the new field on `RegistryObservation`. `src/doctor/observation.rs` (new)
owns `delivery_observation`, the one function that reads the delivery state
through the delivery lane's door.

**The rule.** `src/doctor/delivery.rs` (new) owns the probe and the
phase-to-status rule. `src/doctor/runner.rs` calls it and pushes the finding.

**The callers.** `src/cli/projects.rs` (the doctor CLI and the checker CLI),
`src/api/handlers_changes.rs` (the API doctor handler) and
`src/api/status.rs` (the project status document) fill the observation with
the same function.

**The tests.** The doctor's own suite gains the rule's cases; a new
integration test drives the CLI against a real registry with real delivery
rows.

## Decision Ledger

- **Resolved:** the doctor reads the delivery state; it does not compute it.
  The phase vocabulary stays the delivery lane's, and the doctor reads it
  through `build_delivery_report`, the same door `forge delivery status`
  uses. One vocabulary, one door, one owner.
- **Resolved:** the finding is `delivery-state`, a new entry in the doctor's
  inventory. The existing findings are untouched.
- **Resolved:** a project with no recorded delivery is reported as not
  applicable, not as a warning. The doctor does not know which projects are
  meant to be delivered, and a warning it cannot justify is an alarm it
  cannot back up.
- **Resolved:** `hermora-failed` is a warning, not a failure. The delivery
  lane's own refusal says "deployment stays healthy": only the optional
  enrolment did not complete.
- **Resolved:** an unreadable delivery state is `UNAVAILABLE`, which denies
  health. The doctor's contract already says a check that cannot run is not
  healthy.
- **Implementation authorized (2026-10-20):** the owner directed the trunk
  repair programme and authorized this package in the approved order, after
  `runtime-kind-shared-host` closed the fleet's publishing rule and the
  observe lane.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| The doctor observes the delivery state | The report carries the delivery the lane recorded | One finding, one evidence block | The phase is the delivery lane's | `src/doctor/{model,observation,delivery,runner}.rs` | Healthy; failed; in progress | 2.1–2.3, 3.1–3.4 | The doctor's suite |
| The delivery state is read through one door | Every surface reads the same state | The same finding everywhere | `build_delivery_report` | `src/cli/projects.rs`, `src/api/` | The CLI; the API; the checker | 3.5–3.6 | The integration test |
| Observing a delivery writes nothing | The doctor stays read-only | Nothing changes on disk | Read-only selection | The whole change | The registry bytes are unchanged | 3.7 | The integration test |
