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
entries already moved under `forge/crates/` and the consolidated platform
modules keep their project IDs and resolve to their in-tree declarations in
the Workspace Governance portfolio registry. The path changes are recorded in
`modules/workspace-governance/migration/forge-monorepo-path-reconciliation.json`.

The imported module's Forge conformance work has split its large Python
entry points into focused modules. The current entry points are each below
the 1,000-line source-size baseline (`workspace_check.py` 731,
`bind_gate_commands.py` 475, `product_code_quality.py` 778, and
`ui_conformance.py` 956 lines). The import verifier records those code
adaptations and the corresponding added modules explicitly.

The original source repository is archived. `modules/workspace-governance/`
is the maintained copy; any required source change is made in Forge. Consumer
portability remains separate work and does not change the origin's archive
status.

Run `python3 scripts/check-workspace-governance-import.py` from the Forge root
to compare imported paths, Git modes and blob hashes against the recorded
source revision, allow only declared Forge/Governance adaptations and
additions, and confirm the source-only commits remain ancestors. The Workspace
Governance fleet tests resolve their root from `WORKSPACE_ROOT`; absent that
override they derive the portfolio root from the ordinary Forge module layout.
