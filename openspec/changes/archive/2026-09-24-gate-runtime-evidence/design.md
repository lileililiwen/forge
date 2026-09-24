# Design: Gate runtime evidence

## Ownership and boundaries

Driftwatchdog's Gate owns plan resolution (`gate.toml`,
`.ai-gate/gate.yaml`, profiles, rule-pack identity, blocking policy), child
execution, its own `gate_runs` history and exit semantics. Forge owns:
resolution of which runtime a project declares, one bounded invocation,
normalization of the returned status document into Forge evidence,
journaling under the project's registry identity, and surfacing freshness
on doctor/readiness. Forge never re-decides blocking policy: the sibling
says blocked/passed, Forge reports that attribution.

## Runtime resolution

1. `FORGE_GATE_BIN` if set (explicit).
2. `.project.json` `verification.gate_runtime` → name mapping (currently
   only `driftwatchdog`) → ordered policy-binary probe reused from
   `driftwatch-cli-alignment` (or its standalone equivalent).
3. No runtime declared and none resolvable → the command reports
   `unavailable` with the resolution list; doctor's `gate-evidence` finding
   is `unverified`.

## Invocation and status mapping

`<runtime> gate [--dry-run] --format json`, cwd = project root, stdin null,
timeout default 600s (`--timeout-secs` override), stdout size bound as
elsewhere. The status document maps to:

```text
GateEvidence { runtime, runtime_version, contract, revision,
  aggregate: passed|blocked|failed|unknown,
  checks: [{id, state: pass|fail|skip|not_applicable|unresolved, note}],
  observed_at, dry_run: bool }
```

Exit-code rule: a parseable document wins over the exit code (the sibling
exits non-zero exactly when blocked, and a blocked document is evidence,
not an adapter failure). Non-parseable output, timeout or spawn failure →
`unavailable`; the previous evidence file and journal history stay intact.
Unknown `state` strings become `unresolved`, never `pass`.

## Persistence and journaling

- Evidence: `.forge/gate/<project-id>/evidence.json` atomic tmp+rename,
  one latest record, history lives in the operations table (same split the
  release/deploy planes use).
- Journal: operations row kind `gate`, verdict `done` (aggregate passed),
  `blocked` (aggregate blocked), `failed` (unavailable/timeout), `partial`
  reserved; project id is the real registered id (gate is always
  project-scoped; no synthetic fleet project).
- Revision binding: the git HEAD captured at invocation; evidence whose
  revision != current HEAD renders as stale wherever it is read.

## Surfaces

- `forge gate [TARGET] [--dry-run]` — run; human/JSON; exit code mirrors
  aggregate (0 passed, 1 otherwise) so scripts and hooks compose.
- `forge gate status [TARGET]` — read persisted evidence, annotate
  fresh/stale/absent.
- Doctor: `gate-evidence` finding with the four-way verdict vocabulary.
- Provider matrix: `gate-runtime` row (dry-run against the resolved binary
  when `--live` and `FORGE_PROVIDER_LIVE=1`).
- Readiness: release `checks` may include a `gate` kind bound to the
  captured revision (a later wiring task inside this change, reusing the
  existing check-capture mechanism from release-publishing).

## Security

Captured output passes `policy::redact_credentials` before journal,
evidence file or display; the runtime receives no secrets beyond its own
host environment; argument arrays only, never a shell.

## Verification

- Fixtures: real Driftwatchdog `gate --format json` samples (passed,
  blocked, not-applicable rows) plus malformed/timeout/absent cases.
- Cross-surface: doctor verdict changes with evidence freshness only;
  feature-add and other journals unaffected; MCP tool list unchanged.
- Local dogfood: run `forge gate .` against this repository with
  driftwatchdog installed; record the actual snapshot or the exact blocker.

## Implementation corrections (live evidence, 2026-09-24)

- Captured at driftwatchdog `25811ed` (`tests/fixtures/gate/NOTES.md`):
  the design's `gate [--dry-run] --format json` composition is half
  false. `--dry-run` prints the human plan and exits before the JSON
  writer (reconfirms `../driftwatch/NOTES.md`), so a rehearsal is
  reported as a `PlanPreview` outcome — redacted, bounded, never
  persisted and never journaled — while the evidence surface is the
  real `gate --format json` run (only side effect: one `gate_runs` row
  in the project's own `.driftwatch/` store).
- The real document carries a third top-level status beyond the
  assumed PASS/FAIL: `REVIEW_REQUIRED` (pending review with
  `review_required_blocks`); `blocked` stays the authoritative bit. A
  `REVIEW_REQUIRED` that does not block classifies `unknown`, never
  `passed`.
- Contradiction rule per the deploy-executor precedent: a parseable
  `PASS` document riding a non-zero exit downgrades the aggregate to
  `unknown` instead of fabricating a pass.
- CLI form: `forge gate [TARGET]` (run) and `forge gate status
  [TARGET]` (read) are implemented with a `Vec<String>` positional so
  both proposal spellings parse; `status` is the reserved first token.
