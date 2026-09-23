# Proposal: Forge as an external DriftWatch checker

## Why

Driftwatchdog consumes any external spec tool through a stable checker
protocol: a `driftwatch.toml` `[[checkers]]` entry runs a command that emits
`{"alerts": [{severity, message, source, symbol}]}`. Forge already computes
exactly this kind of evidence — doctor maturity findings, quality-policy
results, governance observations and readiness states — but exposes none of
it in checker form, so no sibling project can register Forge as a spec
checker today. Emitting the alerts envelope lets every project in the
workspace gate on Forge's assessment with zero coupling: Driftwatchdog (and
therefore each project's own Gate) gains portfolio-grade evidence as just
another configured checker, and Forge gains a fleet-wide consumer of its
doctor output without any network surface.

## What Changes

- Add `forge check [TARGET]` which runs the read-only assessment planes
  (doctor findings, selected governance observation, readiness status) and
  prints one Driftwatchdog-compatible checker document to stdout.
- Map severities to the checker vocabulary (`fail`→`error`, `warn`→
  `warning`; passing findings are omitted), with `source` as a project-
  relative path and `symbol` as the policy/finding id.
- Keep stdout machine-pure: the JSON document is the only stdout content;
  diagnostics go to stderr; a valid document (including empty alerts) exits
  zero; only a Forge-side error (unregistered project, unreadable path)
  exits non-zero with the existing typed error envelope.
- Bound the document (max alert count, bounded message length) and redact
  every field through the shared credential pipeline.
- Ship a `driftwatch.toml.example`-style registration snippet in docs and a
  contract fixture proving the envelope parses under the checker protocol
  (unknown-field tolerance, `alerts` always present).

## BFS Impact Map

- **Capabilities:** new `external-checker-emission`; read-only reuse of
  `doctor-maturity-assessment`, `quality-policy-integration`,
  `governance-provider-contract` results.
- **Users and flows:** any sibling project adds one `[[checkers]]` block;
  Driftwatchdog check/gate surfaces Forge findings; no Forge-runtime
  dependency in the monitored project's build.
- **Contracts/data/persistence:** read-only; no registry writes, no journal
  row (a checker run must not mutate the assessed project's state); no new
  persisted types.
- **Integrations/configuration:** the project still needs `forge.yaml` and
  registration for the data planes it exercises; unregistered targets fail
  with the existing typed error, not a fabricated alert.
- **Callers:** Driftwatchdog checker runner (external); CLI help/completions;
  no MCP/API/portal exposure in this change.
- **Failure/boundary behavior:** empty alerts array is a valid success
  document; a `{}` without `alerts` must never be emitted; credential-like
  substrings redacted; oversized output truncated with an explicit summary
  alert.
- **Tests:** envelope contract fixture, severity mapping table, purity of
  stdout, no-mutation check (byte-identical forge.yaml/registry before/after
  a checker run), truncation boundary.
- **Dependencies:** existing doctor/policy/governance/readiness code paths;
  driftwatchdog's stable checker protocol (no sibling change required).
- **Compatibility/security/privacy:** additive command only; argument arrays
  never shells; project-relative paths only, so absolute layout never leaks
  through a checker document.

## Capabilities

- `external-checker-emission`: Forge emits a Driftwatchdog-compatible
  checker alerts document for a target project.

## Non-goals

- Adding a network endpoint, watcher or daemon for pushing findings.
- Exposing `forge check` over MCP/API/portal or journaling its runs.
- Changing what doctor computes; this is a projection, not a new assessment.
- Requiring monitored projects to install Forge (checker registration is
  opt-in per project).

Source: requirement.md §23, §24, §32, §34.
