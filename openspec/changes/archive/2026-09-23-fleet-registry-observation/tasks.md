# Tasks: Read-only fleet observation from the workspace registry

## 1. BFS — Baseline and impact coverage

- [x] Copy the real Workspace Governance `projects.json` shape into a
  fixture and record every field the parser must tolerate.
- [x] Map portal fleet section, `forge list`, and registry join points;
  confirm no existing caller assumes fleet data is local.
- [x] Define the `fleet-registry-invalid` error taxonomy (version,
  duplicate, escape, blank, oversized).
- [x] Add fixture variants: clean, stale mtime, escaping path, duplicate
  id, unknown schema version, symlink escape.

## 2. DFS — Requirement-by-requirement implementation

- [x] Implement `fleet::observe` with canonicalized path confinement,
  per-entry isolation and freshness classification.
- [x] Implement CLI `forge fleet list|status|inspect` (human/JSON) with
  `--workspace-registry`/`FORGE_WORKSPACE_REGISTRY` and `--max-age`.
- [x] Implement local-registry join for `managed|unmanaged` labeling
  without registration writes.
- [x] Wire the portal fleet block with worst-status roll-up rules.

## 3. BFS — Cross-surface regression and completeness

- [x] Hash-compare fixture trees before/after every fleet call (no writes).
- [x] Verify `forge list`, doctor, upgrades and imports behave identically
  with a fleet configured and without.
- [x] Verify CLI and portal report byte-equivalent normalized entries.
- [x] Verify no MCP tool or API route appears for fleet surfaces.

## 4. Verification

- [x] `cargo fmt --all -- --check`; `cargo build`; `cargo test
  --all-targets`; `cargo clippy --all-targets -- -D warnings`.
- [x] `node scripts/check-openspec-change-names.mjs`; `openspec validate
  --all --strict --no-interactive`; `git diff --check`.
- [x] Optional live pass against the real sibling registry on this host;
  record evidence or the exact next action.
