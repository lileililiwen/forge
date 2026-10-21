# Change: release-gate-mandatory

## Why

`forge delivery` and `forge deploy` refuse to move an artifact without fresh,
full-profile, passed Forge Gate evidence bound to the revision they are about
to ship. `forge release` did not. The release lane binds evidence through its
own check model, and the manifest chooses which kinds apply, so a project that
never configured a `gate` check was released with no Gate run at all: the plan
said `check \`gate\` is not configured; release does not require it`, and the
release shipped.

The `gate` check that did exist was a private copy of the rule, and a weaker
one. It read the evidence file itself and checked existence, freshness and
aggregate. It did not check the contract version, the full profile, the
project identity, or that the record was not a rehearsal. One owner already
decides this question for the other two lanes
(`src/gate/ship_gate.rs`); the release lane simply never asked.

The fix is the one the delivery lanes already got: the Gate check always
applies to a release, and the release lane asks the shared rule. The release,
the delivery and the deploy lanes then refuse for the same reasons.

## What Changes

- **The Gate check always applies to a release.** It is not a knob.
  `release.checks` chooses the extra checks a project wants — `doctor`,
  `test`, `driftwatch` — and the Gate evidence is always required on top. A
  manifest that declares `kind: gate` is refused, because that line can no
  longer mean anything.
- **The release lane asks the shared rule.** The private copy in
  `src/release/engine/prepare.rs` is deleted. The release, the delivery and
  the deploy lanes now refuse for the same reasons, on the same evidence.
- **The plan reports the rule's verdict.** The Gate check row carries the
  shared rule's own reason, so the owner reads what the delivery lane reads.
- **The apply step re-checks it.** The apply step already re-runs the
  captured checks at the applied revision; a release whose Gate evidence is
  stale for that revision is refused before any stage runs.

## Package Boundary

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `release-gate-mandatory` | forge | Rust | implemented | this package |
| `release-evidence-non-vacuous` | forge | Rust | modified | the readiness contract now folds in the Gate evidence |
| `release-publishing` | forge | Rust | modified | the preparation contract names the Gate evidence |
| `single-delivery-lane` | forge | Rust | read only | the shared rule is not changed; the release lane adopts it |
| The Gate runtime and its evidence | forge | Rust | read only | `forge gate` still owns execution and persistence |
| The doctor, the test runner and DriftWatch | forge | Rust | read only | their checks are unchanged |

## Non-Goals

- **`forge publish` is unchanged.** It has no check model at all and is
  superseded by the artifact-only deploy lane.
- **The release lane does not run the Gate.** `forge gate` owns execution and
  persistence; the release lane only reads the evidence the project already
  persisted.
- **The other release checks are unchanged.** `doctor`, `test` and
  `driftwatch` stay manifest-controlled, and the release lane still runs them
  the same way.
- **The refusal order in the apply step is unchanged.** The `--confirm` guard
  still runs after the readiness judgement, so a missing `--confirm` is still
  reported the same way it was.
- **The doctor is unchanged.** The doctor's own maturity assessment is a
  separate contract.
