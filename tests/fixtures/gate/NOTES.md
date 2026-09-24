# Gate runtime — live sibling surface notes

Captured 2026-09-24 against the real `driftwatchdog` repository at
commit `25811ed`, binary `target/release/driftwatchdog` (the build
carrying the archived `checker-machine-output`,
`product-quality-gate-contract` and `release-evidence-and-capability-gate`
changes). Scratch projects were created with `driftwatch init`; every
`gate --format json` file in this directory is verbatim sibling output
with no hand edits (`gate-status-pass.json` and
`gate-status-blocked.json` are byte-identical copies of the same real
captures under `../driftwatch/`).

Provenance caution: the copy installed at `~/.cargo/bin/driftwatchdog`
on this host predates `product-quality-gate-contract` and refuses the
`product`/`rust-product` profiles outright. Captures here used the
source-tree release build.

## Documents captured

- `gate-status-pass.json` — real run, exit 0: `status: "PASS"`,
  `blocked: false`, one `PASS` row.
- `gate-status-blocked.json` — real run, exit 1: `status: "FAIL"`,
  `blocked: true`, a `FAIL` row carrying `diagnostic` and
  `remediation` strings.
- `gate-status-not-applicable.json` — real run, exit 0: `status: "PASS"`,
  `blocked: false`, a `PASS` row plus a `NOT_APPLICABLE` row. The
  sibling emits `NOT_APPLICABLE` rows from the missing-command opt-out
  path (`required = false`, no `command`): `missing_evidence` names
  `<id>:command`, `diagnostic` reads
  "no command declared for this concern; no execution was attempted".
- `gate-status-review-required.json` — real run, exit 1: top-level
  `status: "REVIEW_REQUIRED"`, `blocked: true`, `pending_reviews`
  naming the review. This is the THIRD top-level status value Forge
  must classify; the change design only assumed PASS/FAIL.
- `gate-dryrun-plan.txt` — real `gate --dry-run --format json`, exit 0:
  the human plan text, NOT a JSON document. Reconfirms the
  `../driftwatch/NOTES.md` finding that the dry-run surface exits
  before the JSON writer; `--format json` changes nothing there.
  Forge therefore treats a `--dry-run` rehearsal as a plan preview
  (never persisted, never journaled) and only the real
  `gate --format json` run as the evidence surface.

## Shape and vocabulary facts (verified)

- The gate document carries no `contract` field; Forge discriminates
  it by shape (`blocked` + `results`), the same rule the policy plane
  applies.
- Top-level `status` ∈ `PASS | FAIL | REVIEW_REQUIRED` observed at
  `25811ed`; `blocked` is the authoritative blocking bit (a blocked
  aggregate with empty `failures` happens whenever
  `review_required_blocks=true` and a review is pending).
- Per-check `status` ∈ `PASS | FAIL | REVIEW_REQUIRED | NOT_APPLICABLE`
  (the `REVIEW_REQUIRED` row state is the adapter-failure path: spawn
  failure, timeout, signal, malformed envelope or envelope/exit-code
  disagreement — see the sibling `src/gate/adapters.rs`).
- Envelope semantics: a checker-style envelope reporting
  `NOT_APPLICABLE` with exit 0 maps to a `PASS` ROW whose `diagnostic`
  embeds the envelope JSON; the `NOT_APPLICABLE` row state comes only
  from the manifest missing-command path. Forge maps row
  `NOT_APPLICABLE → not_applicable`, `REVIEW_REQUIRED → unresolved`
  (never pass), unknown future strings → `unresolved`.
- Forge's aggregate classification: `blocked=true → blocked`;
  `PASS`+exit0 → `passed`; `PASS`+nonzero-exit → `unknown`
  (contradiction, never a fabricated pass); `FAIL` → `failed`;
  `REVIEW_REQUIRED` without blocking → `unknown` (a pending review is
  not a passed gate); any future status → `unknown`.
- The real run persists exactly one `gate_runs` row into the project's
  own `.driftwatch/state.db` (sibling state, never Forge state); the
  plan preview persists nothing. Neither surface accepts a `--project`
  flag; scope confinement is the invocation working directory.
