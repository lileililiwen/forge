# Proposal: Ship the Forge governance adapter

## Why

Forge exposes a versioned governance-provider contract (v0.1.0: a JSON
request on stdin, a normalized observation on stdout) and advertises
`workspace-governance` as an optional provider id — but no adapter exists
anywhere, so the portfolio's real governance facts
(`workspace_check.py --audit --json`: ERROR/WARN findings, adoption gaps,
per-project codes) never reach Forge's doctor, portal or release surfaces.
Workspace Governance owns those facts; the missing piece is a small
executable shim in this repository that translates one audit result into
one contract observation. This gives the audit a machine consumer without
this repo gaining any UI, and gives Forge real portfolio evidence.

## What Changes

- Add `scripts/forge_governance_adapter.py`: reads an AdapterRequest JSON
  (`contract`, `action: "check"`, `project_id`, `project_path`) on stdin,
  runs the existing audit for that project, and writes exactly one
  observation document to stdout.
- Mapping: project has no ERROR findings → `pass`; any ERROR attributed to
  the project → `fail`; registry/adoption gap for the project → `blocked`;
  project unknown to the registry → `unknown`; adapter-internal failure →
  non-zero exit (Forge classifies that as `unavailable`).
- Echo the requested `project_id` verbatim, carry `protocol_version`
  `0.1.0`, bind `source_revision` to the project's git HEAD when present,
  bound and redact evidence lines (reuse the checker's SECRET_WORDS rule),
  and never write to any file.
- Unit tests with synthetic audit fixtures; a self-contained smoke test
  piping a sample request.
- README gains the adapter under the mandatory-command surface (as an
  optional consumer), and HANDOFF records it in the next evidence cycle.

## BFS Impact Map

- **Capabilities:** new `forge-governance-adapter`; consumers: the existing
  audit checker (read-only), Forge (external).
- **Users and flows:** a workspace operator points Forge at this script;
  Forge governance status reflects real portfolio findings.
- **Contracts/data/persistence:** no changes to `projects.json`, schemas,
  findings vocabulary or existing CLI outputs; the adapter is read-only.
- **Integrations/configuration:** consumes the audit JSON already produced;
  requires only Python 3 already mandated by this repo; no new deps.
- **Callers:** Forge's executable-adapter boundary; CI unchanged.
- **Failure/boundary behavior:** malformed stdin request → exit non-zero
  with a one-line stderr note (never a guessed observation); missing
  project → `unknown` observation, exit zero; audit crash → non-zero exit
  so Forge reports `unavailable`, not a false pass.
- **Tests:** mapping matrix, request validation, redaction, evidence cap,
  read-only guarantee (temp-tree hash compare).
- **Dependencies:** Forge's published contract (external, already archived
  in the Forge specs); no code import from either side.
- **Compatibility/security/privacy:** existing governance commands
  unchanged; evidence strings are codes and messages, secret-word-scrubbed;
  no network, no writes, no secrets read.

## Capabilities

- `forge-governance-adapter`: this repository can answer Forge's governance
  provider contract for any registered project using its own audit results.

## Non-goals

- Owning or mirroring Forge's observation storage or status vocabulary
  beyond the published contract.
- Changing audit finding semantics for this consumer.
- Auto-registering projects or flipping adoption state.
- Supporting Forge contract actions other than `check` yet.

Source: Workspace Governance README ownership statement ("forge is the
portfolio control-plane consumer"); Forge `governance-provider-contract`.
