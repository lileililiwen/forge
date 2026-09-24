# Design: DriftWatch policy adapter alignment

## Ownership and boundaries

Driftwatchdog owns the policy/gate engine, its storage (`.driftwatch/`) and
its CLI grammar. Forge owns the adapter boundary: binary resolution, bounded
invocation, envelope parsing, normalization to PolicyReport, redaction and
journaling. Nothing in this change moves DriftWatch behavior into Forge or
DriftWatch state into the Forge registry.

## Binary resolution

Ordered probe, first executable hit wins:

1. `FORGE_DRIFTWATCH_BIN` (explicit operator choice; unchanged contract).
2. `driftwatchdog` (cargo/installer binary name).
3. `driftwatch` (npm launcher alias).

The version probe (`--version`) stays best-effort (`unknown` on failure).
Fixture tests continue to inject step 1.

## Invocation contract

- Policy run: `<binary> check --format json` with `current_dir(project)`;
  bounded wait as today. This consumes the sibling's `checker-machine-output`
  envelope: `{contract: "driftwatch-checker/0.1.0", tool, version, checkers:
  [{name, status, alerts: [{severity, message, source, symbol}]}], summary}`.
- Gate fallback for projects carrying `gate.toml` or `.ai-gate/gate.yaml`:
  `<binary> gate --format json --dry-run` (no persistence, no side effects).
  - **Corrected at implementation (2026-09-24, live probe).** The sibling's
    `gate --dry-run` branch prints the human-readable plan and returns
    *before* its JSON writer, so no dry-run composition can produce the
    gate status document. Forge therefore runs the real `gate --format
    json`; its only side effect is one `gate_runs` row in the project's
    own `.driftwatch/` store — sibling state, never Forge registry state,
    and the same storage class `driftwatch gate` already writes in the
    sibling's own workflow. The checker surface keeps its genuine
    no-persistence composition `check --dry-run --format json`, which the
    sibling's `checker-machine-output` change guarantees emits the same
    document. Captures and the full surface matrix live in
    `tests/fixtures/driftwatch/NOTES.md`.
- The fabricated `--project <dir>` argument is removed; cwd confinement is
  the scope guarantee, matching how Driftwatchdog resolves its project.

## Envelope → PolicyReport mapping

- Each checker alert becomes a PolicyFinding: `id` = checker name + symbol,
  `category` = the §24 category the checker declares (unknown categories are
  preserved verbatim as strings, per the existing "future taxonomy" rule),
  severity: `error`/`fatal` → fail, `warning` → warn, `info`/`ok` → pass
  (existing PolicySeverity aliases).
- A gate document whose aggregate is blocked maps its failing checks to
  fail findings; it is a `Reported` outcome, not `Unavailable`. A blocked
  gate must lower the doctor verdict while remaining distinguishable from
  "no evidence".
- Contract strings are checked against a supported set; an unknown major
  contract becomes `unavailable` with the version named.

## Doctor presence detection

`detect_driftwatch` gains `driftwatch.toml`, `gate.toml`,
`.ai-gate/gate.yaml` alongside the existing yaml/json names and
`.driftwatch/`. Presence changes evidence lines only, never maturity by
itself.

## Migration and compatibility

Existing fixture scripts remain valid (they are invoked through the same
binary slot and already emit PolicyReport JSON). Real hosts with only the
npm alias, only cargo, or both, all resolve deterministically. Observations
persisted by the previous invocation shape stay valid; no registry migration.

## Verification

- Unit: probe order, mapping table, blocked-gate classification, unknown
  contract refusal.
- Contract fixtures: checker-report and gate-status JSON samples under
  `tests/fixtures/`.
- Live evidence on a host with driftwatchdog installed: provider row becomes
  `supported` with provenance; absent hosts keep `not-run` honesty.
- Regression: full suite plus doctor verdict stability for projects with no
  DriftWatch at all (must stay `unverified`, never `fail`).
