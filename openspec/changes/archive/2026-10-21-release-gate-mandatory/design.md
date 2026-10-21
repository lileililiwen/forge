## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The release lane's captured checks and readiness judgement | `forge` | Yes. `src/release/engine/prepare.rs`. |
| The release manifest's check vocabulary | `forge` | Yes. `src/release/model.rs`. |
| The shared delivery rule | `forge` | Read only. `src/gate/ship_gate.rs`. |
| The Gate runtime, its evidence and its persistence | `forge` | Read only. `src/gate/`. |
| The doctor, the test runner, the DriftWatch adapter | `forge` | Read only. |
| The publish lane | `forge` | Read only. |

## The Gap

Two lanes put an artifact on a target, and they already agreed about what
verification means. `forge delivery` refused without fresh, full-profile,
passed Gate evidence bound to the revision it was about to deliver, and
`forge deploy` adopted the same rule. One owner decides it:

```rust
// src/gate/ship_gate.rs
pub fn evaluate_ship_gate(project_dir: &Path, project_id: &str, revision: &str) -> ShipGateStatus
```

The release lane was the third lane, and it decided differently. It bound
evidence through its own check model, and the manifest chose which kinds
applied:

```yaml
release:
  checks:
    - kind: doctor
    - kind: test
```

With that manifest, the plan reported:

```
  - gate disabled applicable=false revision=f9852ba2457: check `gate` is not
    configured; release does not require it
```

and the release was ready. The owner who had just run `forge gate` and seen
it pass, and the owner who had never run it, got the same release.

The `gate` check that existed was also not the shared rule. It read the
evidence file itself:

```rust
// src/release/engine/prepare.rs, before this change
fn run_gate_check(project_dir: &Path, revision: &str) -> CapturedCheck
```

and decided on existence, freshness and aggregate alone. It never checked
the contract version, the full profile, the project identity, or that the
record was not a rehearsal. So a `fast`-profile run, a rehearsal, or another
project's evidence could back a release claim that the delivery lane would
have refused.

## The Decision

Two changes, taken together.

**The Gate check always applies.** The release lane always captures a Gate
check. `release.checks` chooses the extra checks — `doctor`, `test`,
`driftwatch` — and the Gate is required on top of them.

The manifest vocabulary follows. `kind: gate` is refused, with a message that
says the truth:

```
release.checks kind `gate` is not configurable; the Gate check always applies
to a release
```

A line that can no longer change anything is a lie in the contract. This
programme has removed a good number of them.

**The release lane asks the shared rule.** `run_gate_check` is deleted. The
release check calls `evaluate_ship_gate` and reports its verdict:

| The shared rule says | The release check reports |
|---|---|
| satisfied | `gate pass applicable=true` |
| refused | `gate fail applicable=true`, with the rule's own reason as the detail |

The release lane does not re-classify the refusal. The rule's reason says
whether the evidence was absent, stale, a rehearsal, a fast-profile run, or
another project's record, and the release shows it unchanged. The delivery
lane shows the same words.

## Which Project the Evidence Belongs To

The shared rule compares the evidence's project id with the project it is
asked about. The release lane compares it with the **manifest's** project id,
not the registry id, because that is the id the Gate itself bound the
evidence to: `forge gate` reads the project id from the same manifest.

In the CLI the two are the same value — `src/cli/release.rs` resolves the
registry row and the manifest together and hands the manifest's id to the
engine. Where they could differ, the manifest's id is the one the evidence
was written under, so it is the one that must match.

## Where It Is Written

`collect_captured_checks` in `src/release/engine/prepare.rs` builds one row
per check kind. It keeps that shape:

```rust
for kind in CONFIGURABLE_CHECK_KINDS {
    // doctor, test, driftwatch — declared or reported as disabled
}
// The Gate check is not declared; it always applies.
out.push(run_gate_check(project_dir, manifest, revision));
```

The Gate row stays last, so the plan reads in the same order it always has.

`KNOWN_CHECK_KINDS` becomes `CONFIGURABLE_CHECK_KINDS` and loses `gate`. The
constant still has one job: the capture list and the refusal that names the
declarable kinds cannot drift apart.

## The Apply Step

`apply_release` already re-runs the captured checks at the applied revision
and refuses before any stage runs. It needed no change to become correct:
once the Gate check is always applicable, the apply step re-reads the Gate
evidence at the revision it is about to release, and refuses when it is not
fresh for that revision.

The `--confirm` guard stays where it is — after the readiness judgement. A
release that is not ready is refused with `release-check-failed`; a ready
release without `--confirm` is refused with `release-invalid`. That order is
unchanged, and this change does not move it.

## User Experience and Interface

A release now needs a Gate run, like the other two lanes need one.

```console
$ forge release prepare ./my-app --version 1.2.3
...
checks:
  - doctor pass applicable=true revision=f9852ba2457: doctor is healthy at the configured target maturity
  - test pass applicable=true revision=f9852ba2457: profile test command passed
  - driftwatch disabled applicable=false revision=f9852ba2457: check `driftwatch` is not configured; release does not require it
  - gate fail applicable=true revision=f9852ba2457: Forge Gate evidence is absent for `my-app`; run `forge gate --profile full` from the project directory and retry
ready: no
summary: release plan not ready: failing check(s) gate:fail; release execution blocks before tag or publication
```

The refusal is the delivery lane's refusal, word for word. Nothing else in
the plan changes: the changelog, the stages, the documentation locales and
the other checks are exactly what they were.

The browser view already renders the check rows with their details, so the
web release plan carries the same reason.

## Behavioral Model

| Situation | Outcome |
|---|---|
| The project has fresh, full-profile, passed Gate evidence for its revision | The Gate check passes; the plan is ready when the other applicable checks pass. |
| The project has no Gate evidence | The Gate check fails with the rule's absence reason; the plan is not ready and the release is blocked. |
| The Gate evidence belongs to another revision | The Gate check fails with the rule's staleness reason. |
| The Gate evidence is a `fast`-profile run, a rehearsal, or another project's record | The Gate check fails with the rule's reason. |
| The manifest declares `kind: gate` | The manifest is refused: the Gate check is not configurable. |
| The manifest declares `doctor` and `test` | Those two run as before, and the Gate check runs on top of them. |
| The manifest declares no check at all | Only the Gate check is applicable; the plan is not ready until the Gate has passed. |

## Contract and Compatibility

`release.checks` no longer accepts `kind: gate`. A manifest that declares it
is refused at parse time with a message that names the truth. This is the one
breaking change in the package, and it is deliberate: a knob that cannot
change anything should not exist in a contract.

Everything else in the release contract is unchanged: the stages, the
packages, the containers, the documentation locales, the notes, the
changelog, the identity derivation, the per-stage records, the retry
behaviour and the tag-conflict refusal.

The `release-evidence-non-vacuous` capability is modified where the readiness
judgement is described, and `release-publishing` is modified where the
preparation contract names the evidence it binds. Their canonical scenario
names are kept.

## Failure and Boundary Policy

- **The release lane never runs the Gate.** It reads the evidence the project
  persisted. A missing record is an absence, and the owner is told to run
  `forge gate --profile full` — never handed a verdict it cannot trust.
- **An unreadable record is an absence.** The shared rule already treats a
  corrupt evidence file as absent, and the release lane inherits that.
- **The refusal happens before any stage.** The apply step re-judges the
  evidence at the applied revision and refuses before the commit stage, so no
  tag and no publication happen.
- **The evidence is read, never written.** The release lane writes no
  evidence and no Gate state.
- **A refusal is not a crash.** The plan reports `ready: no` with the reason;
  the apply step refuses with `release-check-failed`.

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

A real run: a project with no Gate evidence is refused by `forge release
prepare`, with a reason that names the Gate. A project with fresh, full
Gate evidence for its revision prepares and applies. The same evidence the
delivery lane accepts is accepted here, and the same evidence it refuses is
refused here.

## Implementation Plan

**The vocabulary.** `src/release/model.rs` stops accepting `kind: gate` and
says why.

**The capture.** `src/release/engine/prepare.rs` keeps the three
manifest-controlled kinds and always appends the Gate check. The Gate check
calls `evaluate_ship_gate(project_dir, &manifest.project.id, revision)` and
reports the verdict.

**The fixtures.** Every release fixture — the engine tests and the three
integration suites — writes a Gate evidence record bound to the fixture's
own revision, the way `src/gate/ship_gate_tests.rs` writes one.

**The tests.** The shared rule's own unit tests already pin every condition.
The release lane adds a test that the Gate check always applies, one that a
`kind: gate` manifest is refused, and one that the release and the delivery
lane refuse for the same reason.

## Decision Ledger

- **Resolved:** the Gate check always applies. It is the trunk; `doctor`,
  `test` and `driftwatch` are the declared extras.
- **Resolved:** `kind: gate` is refused rather than ignored. A line that can
  no longer change anything should not sit in a contract.
- **Resolved:** the release lane calls the shared rule and reports its
  verdict unchanged. The release lane keeps its own error vocabulary; the
  rule keeps its own reason.
- **Resolved:** the evidence is compared with the manifest's project id,
  because that is the id the Gate bound the evidence to.
- **Resolved:** the `--confirm` guard stays after the readiness judgement.
  Moving it would change a refusal that has nothing to do with this change.
- **Resolved:** the publish lane is left alone. It has no check model and is
  superseded by the deploy lane.
- **Implementation authorized (2026-10-21):** the owner directed the trunk
  repair programme and authorized this package in the approved order, after
  `project-changelog-bootstrap` closed the scaffold's changelog gap.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| The Gate check always applies to a release | The plan always carries a Gate check row | The owner cannot skip the Gate | The Gate is the trunk, not a knob | `src/release/engine/prepare.rs` | Every profile, every manifest | 3.1–3.2 | The engine suite |
| The manifest cannot configure the Gate away | `kind: gate` is refused | The owner is told the truth at parse time | A knob that cannot change anything is removed | `src/release/model.rs` | A manifest that declares it | 3.1 | The config suite |
| The release lane uses the shared rule | The same refusal words as the delivery lane | One rule, three lanes | The private copy is deleted | `src/release/engine/prepare.rs` | Absent, stale, rehearsal, fast, other project | 3.2 | The shared rule's unit tests |
| The apply step re-checks the evidence | A stale release is refused before any stage | The release does not ship unverified | The apply step already re-runs the checks | `src/release/engine/apply.rs` | A moved revision | 3.3 | The apply suite |
