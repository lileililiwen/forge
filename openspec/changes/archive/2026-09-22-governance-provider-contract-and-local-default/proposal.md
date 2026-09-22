# Proposal: Standalone-first governance providers

## Why

Forge must be useful when installed alone and must not become coupled to
Workspace Governance, DriftWatchdog, Sisyphusfy, Ariadex, Jenkins, or any
other sibling project. External governance can add portfolio policy and
evidence, but its absence must not prevent Forge's local project workflows.

## What Changes

- Define a versioned, provider-neutral governance observation contract.
- Add a built-in local provider as the default and compatibility baseline.
- Make external governance providers optional, replaceable, and explicitly
  selected per workspace.
- Keep provider-specific payloads opaque to Core while normalizing status,
  source, revision, timestamp, and evidence references.
- Expose provider discovery, selection, status, and inspection through the
  local CLI/Core contract.
- Add a Workspace Governance adapter only as an optional consumer of the
  contract; it is not a Forge build-time or runtime prerequisite.

## BFS Impact Map

- **Capabilities:** Core provider contract; local governance; optional external
  observations; provider selection and inspection.
- **Users and flows:** standalone local use, offline use, external-provider
  use, provider replacement, and provider unavailability.
- **Contracts/data/persistence:** versioned provider descriptor and observation
  DTOs; provider configuration; timestamped observations; no provider-owned
  registry becomes Forge's source of truth.
- **Integrations/configuration:** optional `.forge/providers.yaml` or its
  versioned equivalent; executable adapters use structured argument and JSON
  boundaries. No mandatory sibling path, database, network, or account.
- **Callers:** CLI, MCP, API, portal, doctor, release and deployment surfaces
  consume Core-normalized observations and retain their transport parity.
- **Failure/boundary behavior:** missing, disabled, incompatible, stale,
  unavailable, or replaced providers remain explicit and never become PASS.
  Local commands continue when an external provider fails.
- **Tests:** local-only fixtures, unavailable-provider fixtures, replacement
  provider fixtures, malformed payloads, stale observations, and optional
  Workspace Governance adapter contract tests.
- **Dependencies:** existing manifest/registry, doctor/evidence, provider
  integration, and transport contracts; no new mandatory external runtime.
- **Compatibility/security/privacy:** existing Forge projects remain valid
  without a provider block; provider commands use constrained arguments,
  bounded output and redaction; credentials remain outside Forge manifests.

## Capabilities

- `governance-provider-contract`: Forge can normalize local or external
  governance observations behind a stable contract.
- `local-governance-default`: Forge works without any external provider.
- `governance-provider-selection`: Users can inspect, select, disable, and
  replace an optional provider without rewriting the project registry.

## Non-goals

- Reimplementing Workspace Governance, DriftWatchdog, agent runtimes, or
  deployment systems inside Forge.
- Making `projects.json`, `.project.json`, or any sibling-specific file
  mandatory for Forge operation.
- Building a plugin marketplace, remote governance service, or credential
  broker.
- Allowing an external provider to silently mutate Forge projects, commit,
  archive, push, deploy, or replace local evidence.
- Treating provider presence, registry presence, OpenSpec validation, or build
  success alone as runtime or release verification.
