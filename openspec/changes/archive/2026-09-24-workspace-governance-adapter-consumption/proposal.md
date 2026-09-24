# Proposal: Consume the Workspace Governance adapter end to end

## Why

The governance provider contract (`governance-provider-contract`, v0.1.0)
accepts any executable adapter, and the README already advertises
`forge governance use workspace-governance . --adapter /path/to/adapter` —
but nothing closes the loop: no adapter ships anywhere, the provider id
`workspace-governance` has no packaged default path, and no contract
fixture proves the mapping from Workspace Governance's real audit output
(`workspace_check.py --audit --json` findings with ERROR/WARN levels and
adoption codes) to a normalized observation. The sibling change
`workspace-governance/forge-governance-adapter` ships the adapter; this
change makes Forge a first-class consumer of it and keeps the standalone
guarantee intact.

## What Changes

- Add a known-provider preset for `workspace-governance`: default adapter
  location `<workspace-root>/scripts/forge_governance_adapter.py` resolved
  only from an explicit `--workspace-root` argument or `FORGE_WORKSPACE_ROOT`
  environment value — never an implicit parent-directory scan.
- `forge governance use workspace-governance .` may omit `--adapter` when
  the preset resolves; an explicit `--adapter` always wins.
- Add contract fixtures shaped by the real Workspace Governance audit JSON
  (adopted-clean, ERROR findings, adoption gap, unregistered project) and
  prove observation normalization, status mapping and redaction hold.
- Record a `workspace-governance` live round trip in
  `docs/provider-evidence.md` once the sibling adapter is executable on the
  host; otherwise the row stays honestly `not-run`.

## BFS Impact Map

- **Capabilities:** `governance-provider-contract` (preset + resolution),
  `local-governance-default` (independence preserved).
- **Users and flows:** operators adopting portfolio governance; provider
  switching tests; offline hosts.
- **Contracts/data/persistence:** contract v0.1.0 unchanged; `.forge/
  providers.yaml` may store provider id without an adapter path when a
  preset exists; observation storage unchanged.
- **Integrations/configuration:** sibling checkout path via explicit flag/
  env only; no mandatory sibling presence; Python not required by Forge
  itself (adapter is an external executable behind stdin/stdout JSON).
- **Callers:** CLI governance commands; MCP/API/portal read surfaces
  unchanged (they consume normalized observations).
- **Failure/boundary behavior:** preset unresolved → existing `Unavailable`
  observation naming what to pass; protocol/identity mismatch handling
  unchanged; local provider still default.
- **Tests:** preset resolution matrix, four audit-shaped fixtures, switching
  without manifest/registry rewrites, absolute-path confinement.
- **Dependencies:** sibling change `forge-governance-adapter` for live
  evidence only; contract already archived in Forge.
- **Compatibility/security/privacy:** existing explicit `--adapter` configs
  keep working byte-for-byte; no secret handling change; adapter output
  stays bounded/redacted.

## Capabilities

- `governance-provider-contract`: a packaged, opt-in preset lets Forge
  discover the Workspace Governance adapter location from explicit input
  alone.

## Non-goals

- Vendoring Workspace Governance code into Forge.
- Auto-detecting a workspace root by filesystem scanning.
- Making portfolio governance required for any local workflow.
- Extending the contract to fleet import (see `fleet-registry-observation`).

Source: requirement.md §32, §38.
