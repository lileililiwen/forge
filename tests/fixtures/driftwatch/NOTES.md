# DriftWatch policy adapter — live sibling surface notes

Captured 2026-09-24 against the real `driftwatchdog` repository at
commit `25811ed` (the archived `checker-machine-output`,
`product-quality-gate-contract` and `release-evidence-and-capability-gate`
changes are all in this build), built with
`cargo build --release` and exercised directly:
`target/release/driftwatchdog <subcommand>` in scratch projects created
with `driftwatch init`. Every JSON file in this directory is verbatim
sibling output with no hand edits (unknown-contract derived by bumping
the contract string in `checker-report-alerting.json`).

## Resolved binary names

- Cargo/installer hosts expose the binary as `driftwatchdog`
  (`Cargo.toml` package and bin name); npm hosts alias `driftwatch`.
- `--version` prints `driftwatch <version>` from BOTH names (the clap
  crate id is `driftwatch`), so the second-token version grammar holds
  on either host.
- The copy installed at `~/.cargo/bin/driftwatchdog` on this host
  (mtime 2026-09-20) predates `checker-machine-output`:
  `check --dry-run --format json` exits 2 with
  `stale-binary-rejection.txt` on stderr and empty stdout. Forge's
  ordered PATH probe resolves to it and honestly classifies the run
  `unavailable` (parse failure naming the exit status and stderr),
  never a PASS.

## Surfaces actually supported (verified)

- `driftwatchdog check --dry-run --format json` →
  `checker-report-passing.json` / `checker-report-alerting.json`:
  contract `driftwatch-checker/0.1.0`, `tool: "driftwatchdog"`,
  `version`, `generated_at`, `checkers[]`
  (`name`, `status` ∈ `ok|alerting|failed|timeout|protocol-error`,
  `alerts[]` always present, bounded `error` note on failure rows) and
  `summary` counts. `--dry-run` persists nothing and exits 0
  unconditionally (even with failing rows); a real run persists
  `drift_alerts` rows into the project's own `.driftwatch/state.db`
  and exits 1 (some failed) / 2 (all failed) with the same document.
  The optional top-level `project` field is omitted by this build;
  Forge treats it as optional.
- Alert severities are free-form strings; unknown shapes default to
  the warning bucket on the sibling side. Optional checker-declared
  fields survive under `extra` (e.g. `extra.category` in
  `checker-report-alerting.json`).
- `driftwatchdog gate --format json` (real run) →
  `gate-status-pass.json` (exit 0) / `gate-status-blocked.json`
  (exit 1): `{status, blocked, failures[], pending_reviews[],
  not_applicable[], manifest_digest, rule_pack_version,
  results[]}` where each result is `{gate_id, source, status ∈
  PASS|FAIL|REVIEW_REQUIRED|NOT_APPLICABLE, severity, findings[],
  evidence[], missing_evidence[], diagnostic?, remediation?}`. The
  gate document carries no `contract` field; Forge discriminates it
  by shape (`blocked` + `results`). A real run persists one
  `gate_runs` row into the project's own `.driftwatch/state.db`.
- `driftwatchdog gate --dry-run --format json` → **not** the gate
  document: `gate.rs` early-returns after printing
  `gate-status`-free human plan text (`gate-dryrun-plan.txt`) and
  exits 0, never reaching its JSON writer. The change design's
  assumption of a persisted-nothing JSON composition for `gate` is
  therefore false; Forge's gate surface is the real
  `gate --format json` (see `src/policy/mod.rs::policy_surface_args`).
  The checker surface keeps its genuine no-persistence composition
  `check --dry-run --format json`.
- `gate` in a project without `gate.toml` / `.ai-gate/gate.yaml`
  prints text ("nothing to gate") and exits 0 without a document;
  a `.ai-gate/gate.yaml` targeting a foreign runtime likewise prints
  text and exits 0. Forge only selects the gate surface when a gate
  manifest exists, and both text outcomes classify `unavailable`
  (no parseable document), never PASS.
- Neither subcommand accepts a `--project` flag (that flag never
  existed); scope confinement is the invocation working directory.
- Checker/gate execution requires an initialized sibling project:
  `check` and real `gate` open `.driftwatch/state.db` eagerly and an
  uninitialized directory exits 1 with a sqlite hint and empty
  stdout → Forge honestly reports `unavailable`.
- Checker programs run as argument arrays (no shell splitting):
  `command = "cat"`, `args = ["payload.json"]`. Scratch checker
  projects for reproducing the captures: see the `[[checkers]]`
  blocks mirrored from the capture session; the alerting payload
  carries a deliberately fake `ghp_...` token to prove the
  redaction pipeline end to end.

## Forge classification contract

- Parseable checker or gate document → `Reported` with normalized
  findings, regardless of exit code; only missing binary, timeout,
  or unparseable stdout → `Unavailable`; unknown
  `driftwatch-checker/<other-major>` → `Unavailable` naming the
  contract (see `checker-report-unknown-contract.json`).
- Blocked gate aggregate → FAIL rows become fail findings,
  REVIEW_REQUIRED becomes warn, NOT_APPLICABLE becomes a
  not-applicable pass; never `Unavailable`.
