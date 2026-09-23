# Proposal: Read-only fleet observation from the workspace registry

## Why

Forge's fleet surfaces — `forge list --all`, the portal fleet dashboard,
fleet upgrade scoping — only know projects manually imported into the local
SQLite registry. Workspace Governance already inventories the real portfolio
(`projects.json`: id, path, profile, lifecycle, adoption for the whole
workspace), so Forge's fleet views currently describe an empty or synthetic
fleet while the actual ~70-project portfolio exists one file away. A
read-only mirror gives Forge honest fleet situational awareness and gives
Workspace Governance a second consumer of its registry without either repo
ceding ownership.

## What Changes

- Add `forge fleet list|status|inspect <ID> --workspace-registry <PATH>` (or
  `FORGE_WORKSPACE_REGISTRY`) reading the registry document under its
  `schema_version` contract. (`--registry` names the global local
  registry-database flag, so the workspace document takes its own flag.)
- Surface each entry as a timestamped fleet observation: id, path, declared
  profile/lifecycle/adoption, whether a `forge.yaml` was found at the entry
  path, and registry staleness (file mtime vs `--max-age`).
- Entries are explicitly `unmanaged` unless the same project id exists in
  the local registry; mirroring never registers, imports or mutates them.
- Portal fleet section and `forge list` gain an optional fleet block
  (read-only transports reuse the same Core query).
- Hard read-only guarantees: the registry file and every sibling directory
  are never written; entry paths that escape the registry's workspace root
  (traversal or symlink) are refused as malformed entries.

## BFS Impact Map

- **Capabilities:** new `fleet-registry-observation`; read surfaces of
  `control-plane-portal`, `project-import`/registry (query peers).
- **Users and flows:** portfolio inspection, adoption triage (which
  siblings still lack `forge.yaml`), fleet scoping decisions.
- **Contracts/data/persistence:** no registry-schema change; fleet data is
  re-read on demand (no mirror table), so staleness is explicit not hidden.
- **Integrations/configuration:** Workspace Governance `projects.json`
  schema_version 1 as an input contract; absence → fleet block reports
  `unconfigured`.
- **Callers:** CLI, portal view; MCP exposure deliberately out of scope.
- **Failure/boundary behavior:** malformed JSON, unknown schema version,
  escaping paths, duplicate ids → typed `fleet-registry-invalid` with the
  entry named; never a partial silent drop.
- **Tests:** fixture registry copies (clean/escaping/duplicate/unknown
  version/stale), portal parity, no-mutation assertions on fixture trees.
- **Dependencies:** none on other proposed changes; `project-import`
  unchanged, still per-project and manifest-first.
- **Compatibility/security/privacy:** read-only file access inside an
  operator-declared root; redaction of evidence strings; no scanning of the
  parent filesystem without `--registry`.

## Capabilities

- `fleet-registry-observation`: Forge reports the portfolio declared by an
  external workspace registry without adopting or mutating it.

## Non-goals

- Bulk-importing every registry entry into the local registry.
- Writing adoption status back to Workspace Governance.
- Fleet upgrades over unmanaged projects (import stays the gate for
  mutation).
- MCP/API exposure in this change.

Source: requirement.md §7, §32, §36.
