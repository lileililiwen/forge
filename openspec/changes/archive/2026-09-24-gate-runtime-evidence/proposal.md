# Proposal: Execute the declared gate runtime and journal its evidence

## Why

`.project.json` on this very repository declares
`verification.gate_runtime: "driftwatchdog"` with `evidence_status:
"planned"`, and AGENTS.md/HANDOFF repeat "No shared Gate Runtime is
configured yet" on every evidence block — yet Driftwatchdog ships exactly
that: a Gate that resolves a project's `gate.toml` / `.ai-gate/gate.yaml`
plan, executes checks, persists a snapshot and exits non-zero on blocking
failures. Forge can neither run that gate for a registered project nor
surface its result, so every Forge workflow that should be gated
(completion, release checks, doctor maturity) falls back to manual
assertion. Wiring the declared runtime closes the loop the portfolio
already points at and turns "planned" into evidenced.

## What Changes

- Add `forge gate [TARGET] [--dry-run] [--format json]`: resolves the
  target's declared gate runtime (`.project.json.verification.gate_runtime`,
  falling back to the policy binary resolution), executes the sibling's
  documented `gate [--dry-run] --format json` in the project directory with
  bounded timeout, and maps the status document into a Forge GateEvidence
  record (aggregate verdict, per-check rows, tool version, revision,
  timestamp).
- Journal each run in the operations table (kind `gate`) with
  `done|failed|blocked|unavailable` verdicts and persist the latest
  evidence under `.forge/gate/<project-id>/evidence.json` (atomic write).
- Doctor gains a `gate-evidence` finding: pass on fresh passing evidence,
  warn on stale, fail on a blocked snapshot, `unverified` when no run
  exists or the runtime is absent — never PASS by silence.
- `forge gate status [TARGET]` reads the persisted evidence without
  re-running.
- The provider matrix gains a `gate-runtime` row under the existing
  opt-in live rules.
- Update `.project.json` vocabulary notes and HANDOFF/AGENTS evidence text
  only as documentation consequences (pointer semantics unchanged until an
  implementation cycle selects this change).

## BFS Impact Map

- **Capabilities:** new `gate-runtime-evidence`; consumes
  `quality-policy-integration` (binary resolution, redaction),
  `doctor-maturity-assessment` (new finding),
  `provider-integration-evidence` (new row),
  `profile-and-release-readiness` (release checks may cite gate evidence).
- **Users and flows:** local completion gates, release preparation,
  portfolio-wide assurance that a shared runtime actually ran.
- **Contracts/data/persistence:** versioned GateEvidence record; operations
  table kind `gate`; no schema rewrite; evidence file layout mirrors other
  `.forge/<plane>/` patterns.
- **Integrations/configuration:** Driftwatchdog `gate` CLI surface only
  (manifest discovery belongs to the sibling); timeout default 600s
  (gates can be long); `FORGE_GATE_BIN` override for exotic hosts.
- **Callers:** CLI first; `forge gate` intentionally not exposed over MCP
  yet (operation-bearing tools enter the mature registry only through that
  spec's own classification cycle).
- **Failure/boundary behavior:** missing runtime → `unavailable` finding
  and exit 1 with evidence untouched; blocked gate → Forge exits non-zero
  mirroring the sibling's blocking semantics; stale evidence (revision
  moved) never satisfies a readiness claim; a gate that has never run is
  `unverified`.
- **Tests:** status-document fixture mapping, verdict matrix (pass/blocked/
  timeout/absent/unknown-schema), staleness against revision, journal
  verdicts, redaction of captured output.
- **Dependencies:** soft on `driftwatch-cli-alignment` (shares binary
  resolution); Driftwatchdog's gate JSON exists today, so the sibling
  change `checker-machine-output` is not a prerequisite.
- **Compatibility/security/privacy:** optional external binary behind env
  override like every other adapter; bounded capture; credential
  redaction; no network.

## Capabilities

- `gate-runtime-evidence`: Forge can execute a project's declared shared
  gate runtime and report timestamped, revision-bound evidence across its
  surfaces.

## Non-goals

- Reimplementing gate planning, blocking policy or checks inside Forge
  (Driftwatchdog owns them; requirement.md §24/§32).
- Mandating a gate for any local workflow.
- CI or Jenkins scheduling (the sibling repos and CI own orchestration).
- MCP exposure of `forge gate`.

Source: requirement.md §23, §24, §25, §29, §32.
