# Design: consume the sibling's gate plan rehearsal document

## 1. Ownership and boundary

| Path | Change |
|---|---|
| `src/gate/rehearsal.rs` (new) | the whole consumption: wire types, the shape-detection rule, the invariant gate, the sanitized display struct, `render_rehearsal_human`, `rehearsal_envelope`, the closed verdict-shaped deny-set and its walker. Pure at the mapping layer: no clock, no filesystem, no database, no subprocess, no new dependency. |
| `src/gate/rehearsal_tests.rs` (new) | unit proofs: parse, render, detection rows, degradation, deny-set, journal verdict. |
| `src/gate/mod.rs` | module registration, re-export, and the corrected module documentation (the "exits before the JSON writer" claim is now false). |
| `src/gate/model.rs` | the `GateOutcome::Rehearsal` variant and corrected variant documentation. |
| `src/gate/engine.rs` | detection ordering in `run_gate`; the `journal_verdict` preview/rehearsal arm. |
| `src/cli/functions_13.rs` | the CLI render arm for `GateOutcome::Rehearsal`. |
| `tests/fixtures/gate/*` | one verbatim rehearsal capture + a `NOTES.md` provenance entry. |
| `tests/gate_contract/documented_help.rs` | one end-to-end rehearsal case. |

The rehearsal consumer MUST NOT:

- add a dependency (hand-written serde types only);
- construct, mutate or read a `GateEvidence`, a `GateAggregate` or a
  `GateCheckState` — the types exist so a rehearsal has nowhere to put an
  outcome;
- write a file, spawn a process or open a database;
- call `classify_aggregate`, `save_evidence` or `record_operation`;
- change `parse_gate_document`, `classify_aggregate`, `GateEvidence`,
  `GateAggregate`, `GateCheckState` or the real (non-dry-run) path.

## 2. The producer's document, and its two invariants

From `driftwatchdog` `src/gate/dry_run_plan.rs` (read-only):

> This is a **third document kind**, not a variant of the status
> document … It **carries no verdict-shaped key at any depth**. Not
> `status`, not `result`, not `passed`, not `blocked` … `would_block` is
> intentionally absent — see `BlockingPrediction`.

> `WouldBlock::Undetermined` … Serialized as `null` — **never** `false`,
> because `false` is a claim that nothing ran and passed.

Both invariants are consumer obligations, not producer trivia:

1. **No verdict.** Forge must not manufacture one from the absence of a
   verdict field. A rehearsal therefore has its own outcome variant and
   renders no aggregate anywhere (§5).
2. **Side-effect free.** `executed: false`, `persisted: false`, no
   `gate_runs` row. Forge must not treat the rehearsal as a run for any
   persistence or journaling purpose (§5).

Live capture on this repository (`~/code/driftwatchdog/target/release/driftwatchdog`,
`02a13d4`/`447d1b0`) is stored verbatim as
`tests/fixtures/gate/gate-rehearsal-plan.json`: 9 planned checks, all
required, `source-file-size` bound `in-process` and the rest `manifest`,
every timeout `60000`, `review_required_blocks: true`, exemption budget
`within` (cap 2, declared 0, headroom 2), no unbound required checks,
and `blocking_prediction.kind =
"undetermined-before-execution"` with `would_block: null`.

## 3. The shape-detection rule (normative)

### 3a. Why version cannot serve

Forge probes `runtime_version` through `--version`
(`src/gate/engine.rs::probe_version`) and renders it in every outcome.
It is **not** a discriminator for this decision:

| binary | `--version` | `gate --dry-run --format json` |
|---|---|---|
| `~/.cargo/bin/driftwatchdog` (Oct 5) | `driftwatch 0.1.0` | human plan text |
| `~/code/driftwatchdog/target/release/driftwatchdog` (current) | `driftwatch 0.1.0` | rehearsal JSON document |

The document shape changed and the version string did not. A
version-branch would therefore have to be *guessed* from a build date or
a path — neither of which is available in the sandboxed invocation, and
either of which would silently mis-render the other binary. Worse, the
failure direction is asymmetric: guessing "new" turns human text into a
malformed rehearsal (a degrade, survivable), while guessing "old" keeps
today's raw-JSON display forever. The only honest signal is the payload
itself, which is exactly what the sibling documented it for:

> It exists so a consumer that has learned to recognise *one* rehearsal
> document does not have to infer it from the absence of fields.

### 3b. The rule

A parsed JSON value is a **rehearsal** if and only if **all** of the
following hold. They are ordered cheapest-and-most-decisive first.

1. `contract` is a string whose value starts with
   `driftwatch-gate-plan/` (family match; the patch version is not
   compared, so an additive `0.2.0` still parses and an unrecognised
   family is refused).
2. `document_kind == "gate-plan-rehearsal"`.
3. `dry_run == true`.
4. `executed == false` **and** `persisted == false`.
5. The whole document deserialises into `WireRehearsal` — i.e. every
   field Forge renders is present with the expected JSON type
   (`planned_checks` is an array of objects with a string `id` and a
   boolean `required`, `blocking_prediction` is an object with a string
   `kind`, …).
6. `blocking_prediction.would_block` is **not** `false` — `true` or
   `null` or absent. (A `false` violates the producer's stated
   invariant; treating the document as unrecognised is the fail-closed
   answer, and it can only degrade to a non-verdict preview.)

Any failure returns `None`: "not a rehearsal Forge understands". That is
not an error — it is the documented degrade path (§4).

### 3c. Why this cannot misread either document kind

**A status document can never be read as a rehearsal.** The status
document (`tests/fixtures/gate/gate-status-*.json`) has no `contract`
key at all — its own provenance note says so — no `document_kind`, no
`dry_run`, and no `planned_checks`. Rule 1 alone excludes it, and rules
2–6 exclude it four more times independently. There is no field a status
document carries that could satisfy rule 1: `blocked`, `status`,
`results`, `failures`, `pending_reviews`, `manifest_digest` and
`rule_pack_version` are all inert to this rule.

**A rehearsal can never be read as a status document.** `parse_gate_document`
requires `blocked: bool` + `results: array`, and the sibling's closed
deny-set makes `blocked` and `results` *illegal* at any depth of a
rehearsal document (`VERDICT_LIKE_KEYS`, debug-asserted by the producer
against its own output). So on today's wire both directions are already
safe — and rule (4)/(6) plus the ordering below make them safe even if
that stopped being true.

**Ordering removes the last theoretical overlap.** `run_gate` calls the
rehearsal detector **before** `parse_gate_document` (design §4). A future
sibling that added `blocked` to a rehearsal document would still be
classified as a rehearsal, never as evidence. The reverse order would
have made that a silent evidence promotion.

### 3d. What the detector must never do

- **Never infer from absence.** "has `planned_checks`" or "has no
  `results`" is not a rule: an unrelated JSON document (or a future
  sibling document kind) would match it. The three markers are positive
  self-declarations with a namespaced root `contract`.
- **Never branch on `runtime_version`.** §3a.
- **Never return a partial parse.** If the typed parse fails the whole
  document is unrecognised. A half-understood rehearsal rendered as
  "some checks" would read as an incomplete result, which is an
  assertion Forge has no basis for.

## 4. The degrade path

`run_gate`'s existing preview branch stays exactly where it is and keeps
its current meaning, now explicitly scoped: *the runtime answered a
`--dry-run` with exit 0 and text Forge did not recognise as a rehearsal
document*. It bounds the text to `MAX_PLAN_LINES` lines through the same
`capture_note` redaction the evidence path uses, and returns
`GateOutcome::PlanPreview`.

This covers the stale Oct 5 binary (human text), an unrecognised
rehearsal family (`driftwatch-gate-plan/99.0.0` with renamed fields), a
truncated document, and any non-JSON output. In every case the outcome is
`PlanPreview` or `Unavailable` — neither is `Evidence`, neither produces
an aggregate, and `journal_verdict` gives both the non-outcome word
(§5). The previous test `dry_run_reports_plan_preview_without_evidence`
continues to pin this path unchanged.

## 5. The never-evidence guarantee (normative)

Audited call sites of the rehearsal/preview outcome, exhaustively:

| Call site | Behaviour | Verdict |
|---|---|---|
| `src/gate/engine.rs::run_gate` | a recognised rehearsal returns `GateOutcome::Rehearsal { runtime, runtime_version, rehearsal }`. The struct has **no** `aggregate`, no `checks: Vec<GateCheck>` and no `GateCheckState` anywhere in its type, and the branch is taken **before** `parse_gate_document`, `classify_aggregate` and `capture_revision`. | cannot produce an aggregate |
| `src/gate/engine.rs::save_evidence` | unchanged guard `if evidence.dry_run { Err(GateInvalid { "a dry-run rehearsal is never persisted" }) }` — and unreachable from a rehearsal, which holds no `GateEvidence` at all. | never persisted |
| `src/cli/functions_13.rs::cmd_gate_run` — `Evidence` arm | `save_evidence` is called only when `!evidence.dry_run`; `record_operation` only when `!evidence.dry_run`; the envelope reports `"persisted": …` and `"journaled": !evidence.dry_run`. Unchanged. | no rehearsal row |
| `src/cli/functions_13.rs::cmd_gate_run` — `PlanPreview` arm (existing) | builds a plan-only envelope, calls neither `save_evidence` nor `record_operation`. Unchanged. | no rehearsal row |
| `src/cli/functions_13.rs::cmd_gate_run` — **new** `Rehearsal` arm | identical discipline: the envelope is built from the rehearsal display struct alone, `"persisted": false`, `"journaled": false`, no `save_evidence`, no `record_operation`. | no rehearsal row |
| `src/gate/engine.rs::journal_verdict` | **changed** for the two non-run outcomes: `PlanPreview` and `Rehearsal` now return `"rehearsed"` instead of `"done"`. | not readable as an outcome |
| `src/doctor/probes.rs`, `src/release/engine/prepare.rs`, `src/contract/mod.rs`, `forge gate status` | all read *persisted* evidence via `load_latest_evidence`; a rehearsal is never persisted, so no surface can cite one. | unreachable |

The `journal_verdict` change is the one behavioural edit outside the new
module and it exists because `"done"` is precisely the word a reader (or a
future caller that journals whatever it is handed) takes as "the run
succeeded". `rehearsed` states what happened. It cannot weaken a refusal:
the `Evidence` and `Unavailable` arms are untouched, and `"rehearsed"` is
not a gate verdict vocabulary any surface consumes today (`publish` reads
its own `operations` rows by kind, and nothing reads gate operation state
generically).

### The rendered envelope is proved verdict-free

`rehearsal_envelope` builds the JSON the CLI emits, so a unit test can
walk the real payload against `REHEARSAL_VERDICT_DENY_KEYS` — the
sibling's closed deny-set, mirrored here rather than imported, because
the two repositories cannot share code: `status`, `result`, `results`,
`passed`, `blocked`, `outcome`, `aggregate`, `exit_code`, `gate_status`,
`ok`, `success`, `failures`, `pending_reviews`. The test asserts the
emitted envelope contains **none** of them at any depth. `would_block` is
deliberately not on the set (it is a prediction, and it is `true` or
`null`, never `false` — §3b rule 6 refuses the third case).

## 6. Rendered shape

Human (`render_rehearsal_human`) — plan identity, the checks, the
prediction:

```
rehearsal: driftwatch-gate-plan/0.1.0 (gate-plan-rehearsal) | execution mode: dry-run | nothing was executed, persisted or journaled
plan: version 1 | profile rust-product | rule pack workspace-governance@1 | manifest sha256:53102e32f760136b
blocking policy: review_required_blocks=true | default timeout: 60000ms | source size max_lines: 1000
checks (9 planned):
  - build [required] command=manifest selected-by=manifest timeout=60000ms
  - source-file-size [required] command=in-process selected-by=manifest timeout=60000ms
exemption budget: cap=2 declared=0 headroom=2 required=9 verdict=within
unbound required checks: none
not scheduled: a11y, browser, capability-conformance, release-evidence, responsive
blocking prediction: undetermined-before-execution — would_block=unknown (the bound commands decide); a plan-level prediction from the resolved plan alone, never an observed outcome
```

Every line is built from strings that already passed `capture_note`
(credential redaction, project-path scrubbing, `MAX_NOTE_CHARS` bound),
and the planned-check block is truncated at `MAX_PLAN_LINES` rows with an
explicit `… N more planned checks not shown` line so the bound is never
silent. Checks are printed in document order; nothing is sorted or
deduplicated, because the sibling's order is the resolved plan's.

JSON — the existing `forge gate` envelope, one additive key:

```json
{
  "contract": "0.1.0",
  "dry_run": true,
  "gate": {
    "project": "forge",
    "runtime": "driftwatchdog",
    "runtime_version": "driftwatch 0.1.0",
    "persisted": false,
    "journaled": false,
    "plan": ["<rendered plan lines>"],
    "rehearsal": { "...": "the typed, bounded rehearsal document" }
  }
}
```

`plan` is kept alongside `rehearsal` so a consumer that only ever read
`gate.plan` keeps finding a plan (compatibility §7) and the existing
`tests/gate_contract/documented_help.rs` assertion on
`value["gate"]["plan"][0]` keeps its meaning.

## 7. Compatibility

- **The real run path is byte-identical.** `parse_gate_document`,
  `classify_aggregate`, `GateEvidence`, `GateAggregate`,
  `GateCheckState`, `GateFreshness` and every persisted field are
  untouched; the rehearsal branch returns before the status parse, so for
  a status document the control flow is the same statements in the same
  order as before. A unit test pins that each real capture still maps to
  the same aggregate and rows *and* is not detected as a rehearsal.
- **Additive JSON.** `gate.rehearsal` is new; `dry_run`, `persisted`,
  `journaled` keep their values; `gate.plan` keeps existing meaning.
- **Older siblings keep working.** A binary that emits human text, an
  unrecognised rehearsal family, a truncated document and non-JSON output
  all land in the unchanged plan-line preview. No version branch means no
  version-shaped regression on either binary.
- **`GATE_CONTRACT_VERSION` stays `"0.1.0"`**: it versions Forge's own
  persisted `GateEvidence` record, which this change does not alter, and
  bumping it would invalidate every persisted evidence file.
- **Forward compatibility inside the family**: additive sibling fields are
  ignored by serde; a *non-additive* change fails the typed parse and
  degrades to plan lines — visible, honest, and still never a verdict.

## 8. Security and privacy boundaries

- Unchanged: bounded argument-array invocation, null stdin, the
  `MAX_GATE_OUTPUT_BYTES` stdout bound, the bounded wait, no shell, no
  credential in the invocation.
- The sibling's document is untrusted input. Every captured string —
  contract, document kind, execution mode, profile, rule pack, manifest
  digest, check ids, `selected_by`, `command_source`, blocking rule,
  prediction kind and note, notice, policy words, exemption verdict — is
  passed through `capture_note` (`redact_credentials` + project-path
  replacement + `MAX_NOTE_CHARS` truncation with a visible `[truncated]`
  marker) before it reaches the terminal, the JSON envelope or any test
  fixture comparison. Numbers are bounded by construction (JSON numbers
  are finite; a non-integer where a `u64` is required fails the parse and
  degrades).
- Row counts are bounded: planned-check rows by `MAX_PLAN_LINES`,
  reasons and ids by `MAX_NOTE_CHARS` each, preview lines by the existing
  `MAX_PLAN_LINES`.
- No new process, filesystem, network, credential or authorization
  boundary. No MCP tool, API route, portal section, journal row or
  generated project file.

## 9. New refusals (both fail-closed, neither weakens an existing one)

| Situation | Outcome | Reason wording |
|---|---|---|
| A recognised rehearsal document answers a **non-dry-run** invocation | `Unavailable` | "gate runtime `X` answered a non-dry-run gate invocation with a plan rehearsal document; a rehearsal executes nothing and is never evidence" |
| A document carries rehearsal markers but contradicts them (`executed: true`, `persisted: true`, or `would_block: false`) | `Unavailable` when the exit was non-zero; otherwise the plan-line preview | the same contradiction reason when the status was non-zero, else the preview fallback |

The second row is deliberately *not* a refusal on exit 0: a rehearsal
that fails its own invariant is still a `--dry-run` answer, and refusing
it would make a `--dry-run` fail where it previously previewed. The
contradiction is reported by degrading to the plan-line preview, which
shows the operator exactly what the runtime said and claims nothing. This
is recorded rather than left implicit because "fail closed" has two
different honest answers depending on whether a run was requested.

## 10. Verification

- `cargo test --lib gate::` — the typed parse of the verbatim capture;
  the render (planned checks, command sources, timeouts, policy,
  digest, rule pack, prediction wording including `would_block=unknown`);
  the detection table (each of the six rules, each a row that must
  return `None`); deny-set proof over the real emitted envelope; the
  never-evidence proofs (`run_gate` returns `Rehearsal`, never
  `Evidence`; `journal_verdict` is `rehearsed`; no evidence file is
  created); the degrade rows (unknown family, renamed required field,
  `would_block: false`, human text); the real-run-contradiction refusal;
  and real-document parity — each of the three verbatim status captures
  still maps to the same aggregate/rows through `parse_gate_document`
  and is never detected as a rehearsal.
- `cargo test --lib` — no other module depends on the changed signatures.
- `tests/gate_contract/documented_help.rs` — the end-to-end rehearsal
  case: rendered envelope, no `.forge` tree, no journal row.
- `cargo build`, `cargo fmt --check`, `cargo clippy` (default targets),
  `scripts/contract-parity.sh`, the name checker, the governance
  checker, `openspec validate --all --strict --no-interactive`,
  `git diff --check`.

## 11. Open decisions recorded, not silently fixed

- `bound_note` vs `capture_note`: the rehearsal render uses
  `capture_note` (which also replaces the project's own path with
  `<project>`), because the sibling's document can echo the assessed
  project directory the same way a diagnostic can. The preview path
  already did this; nothing is changed there.
- `MAX_PLAN_LINES` (40) is reused for planned-check rows rather than
  inventing a second bound. A plan with more than 40 checks renders 40
  rows and an explicit remainder line; the JSON envelope keeps every row
  it parsed (the JSON is machine consumption, and its bytes are already
  bounded by `MAX_GATE_OUTPUT_BYTES` upstream). The asymmetry is
  deliberate and is stated here: the *human* surface is bounded, the
  *machine* surface is complete.
- The stale `~/.cargo/bin/driftwatchdog` (Oct 5) remains on PATH and
  still emits human text, so `forge gate --dry-run` without
  `FORGE_GATE_BIN` keeps the plan-line preview on this host. Reinstalling
  the sibling binary is an environment action outside this repository.
