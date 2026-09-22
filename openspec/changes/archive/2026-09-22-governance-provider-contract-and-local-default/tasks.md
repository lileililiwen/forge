# Tasks: standalone-first governance providers

## 1. BFS — Baseline and impact coverage

- [x] Map current doctor, provider, registry, operation, CLI, MCP, API, and
  portal observation types and identify the smallest normalized contract.
- [x] Confirm the local provider remains usable with no `.forge/providers.yaml`,
  no sibling checkout, no network, and no external binary.
- [x] Define the provider configuration schema, protocol version, normalized
  statuses, evidence receipt fields, and replacement semantics.
- [x] Add fixtures for local, unavailable, malformed, stale, identity-mismatch,
  and replacement-provider observations before implementation.
- [x] Record the Workspace Governance adapter as optional and verify that its
  files and command paths are absent from the base-provider dependency graph.

## 2. DFS — Requirement-by-requirement implementation

- [x] Implement the versioned provider descriptor, request, response,
  normalized observation, and typed compatibility errors.
- [x] Implement the built-in local provider and its default resolution path.
- [x] Implement optional provider configuration loading with path, timeout,
  enabled-state, and protocol validation.
- [x] Implement bounded adapter execution with argument arrays, output limits,
  timeout classification, redaction, and identity binding.
- [x] Implement provider list/status/use/inspect Core and CLI operations without
  changing project manifests or registry identity.
- [x] Persist provider source, protocol version, revision, timestamp, status,
  and evidence references as observations with historical replacement safety.
- [x] Add the optional executable-adapter boundary for Workspace Governance
  without importing its implementation or making it mandatory.
- [x] Expose the normalized provider result through CLI, MCP, API, and portal
  transport paths.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify all existing local project commands work after removing provider
  configuration and external adapter binaries.
- [x] Verify disabled, unavailable, stale, incompatible, and identity-mismatch
  results cannot render as healthy or PASS in doctor and portal views.
- [x] Verify switching providers preserves project identity, manifest bytes,
  registry records, historical observations, and operation journal entries.
- [x] Verify CLI, MCP, API, and portal expose the same normalized observation
  and do not expose provider secrets or unbounded output.
- [x] Verify no provider-specific file, status, command, or dependency leaks
  into the local-provider path.
- [x] Review requirement, architecture, security, compatibility, and all
  affected callers for missing boundary scenarios and placeholders.

## 4. Verification

- [x] Run the exact formatting, build, unit, contract, and cross-surface tests
  introduced by the implementation.
- [x] Run the standalone fixture matrix with external adapters and network
  access unavailable; record the actual result.
- [x] Run the optional generic executable-adapter fixture tests separately;
  do not claim live provider verification without the provider environment.
- [x] Run `node scripts/check-openspec-change-names.mjs`.
- [x] Run `openspec validate --all --strict --no-interactive`.
- [x] Run `git diff --check` and review all added files.
- [x] Record the intentionally unrun live external-provider check: run the
  adapter in its owning governance project when that environment is available;
  local and generic adapter evidence do not claim live sibling-provider proof.
