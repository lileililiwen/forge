# Workspace Governance in Forge

Forge now carries Workspace Governance as the ordinary Python project at
`modules/workspace-governance/`. Its Python 3.11 runtime, tests, scripts,
schemas, templates, project declarations, documentation, OpenSpec history,
and configuration remain rooted in that directory. Run its own checks with
that directory as the current working directory, following its `AGENTS.md`
and `README.md`.

The imported source revision is
`2c766c64eb54560fa898115dec27c32abbf4968e`. The import preserves the six
source commits that were ahead of the origin's `main` branch at that time.
They remain available in Forge's Git history. The source directory is ordinary
tracked content; Forge does not use a Git submodule.

The source import does not yet mean that all consumers are migrated. Forge's
provider adapter, legacy project registry readers, vocabulary sync, pinned
standard assets and generated concern shims still have sibling-layout
compatibility references. Those are explicit follow-up work. The four tooling
entries already moved under `forge/crates/` keep their IDs and now resolve to
their new declarations in the Workspace Governance portfolio registry. The
path changes are recorded in
`modules/workspace-governance/migration/forge-monorepo-path-reconciliation.json`.

The imported module Gate currently finds no source-size candidates because
its Python implementation lives in `scripts/`; four production scripts exceed
the 1,000-line Forge baseline: `workspace_check.py` (4,405),
`bind_gate_commands.py` (2,493), `product_code_quality.py` (1,763), and
`ui_conformance.py` (1,096). The Gate passing does not claim those files meet
the baseline. A dedicated conformance package is queued before consumer
migration.

The source origin is not archived until Forge and sibling consumers no longer
require that checkout and every consumer has passed its isolated-project
checks.

Run `python3 scripts/check-workspace-governance-import.py` from the Forge root
to compare imported paths, Git modes and blob hashes against the recorded
source revision, allow only declared monorepo registry/test/path and
whitespace adaptations, and confirm the source-only commits remain ancestors. The Workspace
Governance fleet tests resolve their root from `WORKSPACE_ROOT`; absent that
override they derive the portfolio root from the ordinary Forge module layout.
