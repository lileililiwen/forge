# Proposal: Align the DriftWatch policy adapter with the real CLI surface

## Why

Forge's driftwatch-policy boundary invokes `driftwatch check --project <dir>
--format json` (docs/provider-evidence.md, `src/policy/mod.rs`). The sibling
Driftwatchdog CLI does not expose that surface: `check` accepts only `--only`
and `--dry-run` (no `--project`, no `--format`), machine-readable output lives
on `gate --format json`, and a cargo-installed host exposes the binary as
`driftwatchdog` while only the npm launcher aliases `driftwatch`. As a result
the adapter can never succeed against the real binary, the provider matrix row
stays `not-run`/`unavailable`, and the README's "real provider round trips
remain separately qualified" caveat cannot close. Doctor presence detection
also misses Driftwatchdog's actual config files (`driftwatch.toml`,
`gate.toml`, `.ai-gate/gate.yaml`), so monitored sibling projects look
undriftwatched.

## What Changes

- Resolve the policy binary by ordered probe: `FORGE_DRIFTWATCH_BIN` override,
  then `driftwatchdog`, then `driftwatch`. No PATH assumption about one name.
- Invoke the real surface: the project directory is the working directory
  (already the case); drop the fabricated `--project` argument; consume the
  versioned checker-report envelope from `driftwatch check --format json`
  (driftwatchdog change `checker-machine-output`) and fall back to
  `driftwatch gate --format json` for gate-bearing projects.
- Normalize a blocked/non-zero gate document into findings (severity fail),
  never into `Unavailable`; only missing/timeout/unparseable remains
  `unavailable`.
- Extend doctor DriftWatch presence detection to `driftwatch.toml`,
  `gate.toml`, `.ai-gate/gate.yaml` and `.driftwatch/`.
- Update `docs/provider-evidence.md` to the corrected command surface.

## BFS Impact Map

- **Capabilities:** `quality-policy-integration` (adapter contract),
  `doctor-maturity-assessment` (presence detection),
  `provider-integration-evidence` (driftwatch-policy row semantics).
- **Users and flows:** doctor policy plane, release checks that run
  DriftWatch, provider matrix live runs, sibling projects adopting Forge.
- **Contracts/data/persistence:** unchanged PolicyReport/observation schema;
  new envelope-parsing mapping; observation freshness and journal rows
  unchanged.
- **Integrations/configuration:** real Driftwatchdog binary on PATH;
  `FORGE_DRIFTWATCH_BIN` keeps working; fixture scripts remain the sandbox
  default for tests.
- **Callers:** CLI `forge doctor`, release stage checks, provider runner;
  MCP/API/portal read the same observations (transport parity preserved).
- **Failure/boundary behavior:** absent binary → `unavailable`; blocked gate
  with parseable JSON → reported fail findings; timeout → `unavailable`;
  unknown contract version → `incompatible`/`unavailable`, never PASS.
- **Tests:** envelope fixtures (checker-report and gate-status), ordered
  binary probe, exit-code classification, redaction, stale handling.
- **Dependencies:** driftwatchdog change `checker-machine-output` provides
  `check --format json`; gate JSON exists today as fallback.
- **Compatibility/security/privacy:** no new execution surface (argument
  arrays, bounded waits, no shell); existing fixture-based tests keep passing
  through the override; no sibling code imported.

## Capabilities

- `quality-policy-integration`: Forge drives the actual Driftwatchdog CLI
  surface and classifies its outcomes honestly.
- `doctor-maturity-assessment`: Doctor recognizes real DriftWatch workspace
  markers.

## Non-goals

- Implementing any DriftWatch policy engine inside Forge (requirement.md §24
  forbids replacement).
- Requesting new flags from Driftwatchdog beyond `checker-machine-output`;
  no `--project` flag is added to the sibling.
- Making DriftWatchdog a build or runtime dependency of Forge.
- Changing the PolicyFinding/PolicyReport schema or registry storage.

Source: requirement.md §23, §24, §32, §41.
