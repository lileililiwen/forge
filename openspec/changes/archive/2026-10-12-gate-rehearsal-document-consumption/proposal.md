# Proposal: consume the sibling's gate plan rehearsal document

## Why

`driftwatchdog` landed `gate-dry-run-json-plan` (commits `02a13d4`,
`447d1b0`). `driftwatch gate --dry-run --format json` now emits a
**machine-readable rehearsal document** on stdout instead of human text.
It is a distinct third document kind, not a variant of the status
document: composed from the resolved plan and the manifest alone, with
`contract: "driftwatch-gate-plan/0.1.0"`, `document_kind:
"gate-plan-rehearsal"` and `dry_run: true`.

Forge was not updated, so `forge gate --dry-run` on this repository now
prints the raw JSON indented as plan lines. Verified live against the
current sibling build
(`FORGE_GATE_BIN=~/code/driftwatchdog/target/release/driftwatchdog`):

```
dry-run: gate plan preview for forge (nothing was executed, persisted or journaled)
runtime: driftwatchdog (driftwatch 0.1.0)
  {
    "blocking_policy": {
      "review_required_blocks": true
    },
    "blocking_prediction": {
      ...
```

`src/gate/engine.rs:316-329` falls into the plan-line preview because
`parse_gate_document` requires `blocked` + `results`, which a rehearsal
document does not carry (correctly — it must not look like a verdict).
The fallback was written for a sibling that had no JSON composition at
all; it is now a display defect, not a safety defect: nothing is
persisted, nothing is journaled and nothing claims a verdict.

Two stale comments now assert something false and must be corrected:

- `src/gate/mod.rs:13-21` states the dry-run surface "prints a
  human-readable plan and exits before the JSON writer". No longer true.
- `src/gate/model.rs:117-131` describes the preview fallback as if the
  sibling had no JSON composition, and documents `Evidence` as able to
  carry a rehearsal (`dry_run: true`) — a rehearsal now has its own
  outcome variant, so that sentence is both wrong and under-specified.

## What Changes

- **A typed consumer for the rehearsal document** (new
  `src/gate/rehearsal.rs`): hand-written serde types for the rehearsal
  root, `planned_checks[]`, `blocking_prediction`, `exemption_budget`,
  `blocking_policy` and `source_size_policy`, with **no new dependency**.
- **Shape-based detection, deliberately not version-based.** A document
  is a rehearsal only when all three independent markers agree — a root
  `contract` in the `driftwatch-gate-plan/` family, `document_kind ==
  "gate-plan-rehearsal"`, and `dry_run == true` — *and* the whole
  document parses into the typed struct *and* its self-declared
  side-effect invariants hold (`executed: false`, `persisted: false`,
  `would_block` never `false`). Anything else degrades to today's
  plan-line preview, which is never a verdict either.
- **Detection runs before the status-document parse**, so a payload
  carrying rehearsal markers can never be normalised into
  `GateEvidence` even if a future sibling added a `blocked` bit to a
  rehearsal document.
- **A dedicated `GateOutcome::Rehearsal` variant**, so every rendering
  and journaling call site is forced by the compiler to acknowledge a
  rehearsal instead of inheriting plan-line handling.
- **A proper render** (human + JSON): planned checks with
  required/optional, bound command source, per-check timeout, blocking
  policy, manifest digest, rule pack, profile, plan version, exemption
  budget, unbound required checks, not-scheduled list and the blocking
  prediction with its "unknown ≠ pass" wording — no raw JSON lines.
- **Never evidence, enforced three ways.** `run_gate` builds a
  rehearsal outcome with no `GateEvidence`, no `GateAggregate` and no
  `classify_aggregate` call; `save_evidence` keeps refusing
  `dry_run: true`; and `journal_verdict` changes the preview/rehearsal
  arm from `done` to `rehearsed`, so no rehearsal can be read as a
  completed run even by a future caller that journals the outcome.
  The rendered envelope is proved by test to carry no verdict-shaped key
  at any depth (the sibling's own closed deny-set).
- **Two new refusals, both fail-closed**, for cases that cannot occur
  today: a rehearsal document returned to a non-dry-run invocation, and
  a rehearsal that contradicts its own side-effect markers. Both are
  `GateOutcome::Unavailable`, never evidence and never a pass.
- **Canonical spec update.** `gate-runtime-evidence` gains a
  "Plan rehearsal document consumption" requirement with scenarios for
  rendering, for never-evidence, for unrecognised-shape degradation and
  for an unchanged real status document.

## BFS Impact Map

- **Requirements:** `gate-runtime-evidence` — "Honest surface
  projection" (a rehearsal is now a first-class surface rather than
  scraped plan lines) plus a new "Plan rehearsal document consumption"
  requirement. `gate-evidence-export-consumption` is **not** amended: it
  governs the `forge gate evidence` export plane (its own untrusted
  input contract), not `forge gate`'s invocation of the sibling.
- **Concepts/vocabulary:** `GateEvidence`, `GateAggregate`,
  `GateCheckState`, `classify_aggregate`, `GateFreshness` and
  `MappedDocument` are **unchanged** — a rehearsal must never reach any
  of them. New vocabulary: `GateRehearsal`, `RehearsalPlannedCheck`,
  `RehearsalBlockingPrediction`, `RehearsalBlockingReason`,
  `RehearsalExemptionBudget`, `RehearsalBlockingPolicy`, plus the
  `GateOutcome::Rehearsal` variant.
- **Modules:** new `src/gate/rehearsal.rs` (types, detection, render,
  envelope) and `src/gate/rehearsal_tests.rs`; `src/gate/mod.rs` (module
  registration + corrected module docs); `src/gate/model.rs` (the
  `Rehearsal` variant + corrected variant docs); `src/gate/engine.rs`
  (detection ordering in `run_gate`, `journal_verdict`); `src/cli/functions_13.rs`
  (the CLI render arm). `src/gate/evidence.rs` is **untouched**.
- **Contracts/data:** `GateEvidence` JSON, `GATE_CONTRACT_VERSION`, the
  `.forge/gate/**` layout, the journal schema and every generated
  project tree are untouched — a rehearsal writes none of them. One new
  test fixture: `tests/fixtures/gate/gate-rehearsal-plan.json` (a
  verbatim sibling capture; `tests/fixtures/gate/NOTES.md` records its
  provenance and marks the older human-text capture as the stale-binary
  shape).
- **Callers:** `forge gate` and `forge gate --dry-run`
  (`src/cli/functions_13.rs::cmd_gate_run`) is the **only** consumer of
  `GateOutcome`; `doctor` (`src/doctor/probes.rs`), release prepare
  (`src/release/engine/prepare.rs`), readiness and
  `forge contract emit gate-result` (`src/contract/mod.rs`) all read
  *persisted* evidence through `load_latest_evidence`, which can never
  contain a rehearsal. The MCP registry, API route table, portal
  sections and command catalog are untouched.
- **Persistence:** none added. `save_evidence` still refuses
  `dry_run: true`; the rehearsal path never calls it and never writes a
  journal row (`cmd_gate_run` already guards both on `dry_run`).
- **Integrations:** `driftwatchdog` remains the producer. The version
  probe cannot serve as a discriminator — both binaries report
  `driftwatch 0.1.0` (the stale Oct 5 copy at `~/.cargo/bin/driftwatchdog`
  emits human text, the current build emits the JSON document), so
  detection is by payload shape only.
- **Failures:** every existing refusal is preserved byte-for-byte
  (unparseable output, timeout, spawn failure, non-UTF-8 stdout, output
  bound, unresolvable runtime, undeclared runtime, corrupt persisted
  evidence, `--timeout-secs` bounds, `evidence --dry-run`). Two new
  fail-closed `Unavailable` reasons are added (contradictory rehearsal
  markers; a rehearsal answering a real run). No typed error variant is
  changed, renamed or weakened.
- **Tests:** `cargo test --lib gate::` covers parsing, rendering,
  detection, degradation, never-evidence and real-document parity;
  `tests/gate_contract/documented_help.rs` gains one end-to-end
  rehearsal case (rendered + no `.forge` tree + no journal row).
- **Compatibility:** the JSON envelope of `forge gate --dry-run` gains an
  additive `gate.rehearsal` object alongside the existing
  `gate.plan` for the human-text shape; `gate.persisted` and
  `gate.journaled` stay `false`. A consumer that reads only
  `gate.plan` still finds a plan (the rehearsal renders its planned
  checks there too — see design §4), so the envelope is not broken.
  The real run path emits byte-identical bytes.
- **Concerns:** `quality.md` (success, failure and boundary evidence for
  each new behaviour; unknown/unavailable are not healthy);
  `security.md` (the sibling's document is untrusted input: every
  captured string is redacted and character-bounded before rendering,
  planned-check rows are bounded, no shell, no new process or
  filesystem boundary).

## Capabilities

- **`gate-runtime-evidence`** (modified): shape-detected consumption and
  honest rendering of the sibling's plan rehearsal document, and the
  never-evidence guarantee that keeps a rehearsal out of the persisted
  record, the journal and every aggregate.

## Non-goals

- **Widening `GateCheckState` or `GateAggregate`.** A rehearsal describes
  what *would* run; it has no per-check outcome, so there is no honest
  value to add.
- **Reading `blocking_prediction` as an outcome.** The prediction is
  rendered as a plan-level statement with its own wording; it never
  becomes an aggregate, and `would_block: null` renders as "unknown",
  never "pass".
- **Changing `src/contract/mod.rs` or the `platform.gate-result`
  work.** `emit_gate_result` reads persisted evidence only and cannot
  see a rehearsal; it is out of scope and untouched.
- **Changing `src/gate/evidence.rs`** or its adversarial untrusted-input
  contract. That is the *export* plane's own parser.
- **Version-based detection.** The sibling's `--version` string did not
  change when the document shape changed; a version branch would be a
  false signal (design §3).
- **Adding a dependency** (no `serde_path_to_error`, no JSON-Schema
  crate): hand-written serde types only.
- **Fixing the two pre-existing gate blockers** — the `forge cap`
  command-catalog coverage gap
  (`catalog_has_exactly_one_row_per_clap_path`) and the
  `declared-verification` / `tests` 60s adapter timeouts. Both
  pre-date this change, are unrelated to it, and are recorded with
  their remediation paths instead.
- Phase: consumption-side hardening of an already-shipped sibling
  surface. No publish, no push, no release.
