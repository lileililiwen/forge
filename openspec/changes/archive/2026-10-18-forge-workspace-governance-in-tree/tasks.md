# Tasks: forge-workspace-governance-in-tree

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Reconfirm source HEAD, clean worktree, 400 tracked paths, six
  source-only commits, Python version, commands, and the exact Forge call
  sites listed in the proposal before changing files.
- [x] 1.2 Verify implementation handoff: source project root, Python 3.11
  checks, destination, Git-history preservation, package exclusions, and
  verification commands are explicit.
- [x] 1.3 Add path/blob/mode and ancestry parity checks or a deterministic
  import-verification script; prove it fails on a missing/changed path.
- [x] 1.4 Align proposal, design, tasks, capability requirement and roadmap
  package state before import.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Import source history and all tracked Workspace Governance files as
  ordinary files under `modules/workspace-governance/`; retain all six
  source-only commits and do not create a Git submodule.
- [x] 2.2 Update Forge README and architecture to describe the maintained
  subproject and distinguish its independent Python boundary from the Forge
  Gate and generated-project runtime.
- [x] 2.3 Update the roadmap retirement gate to include Workspace Governance,
  its six local-only commits, and consumer portability; record the import
  revision and non-archived origin accurately.
- [x] 2.4 Reconcile the four consolidated tooling paths in the portfolio
  registry and record/test the ID-preserving path migration.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Compare source tracked paths to imported paths, verify all
  undeclared blob/mode differences fail, and prove each source-only commit is
  an ancestor of the import.
- [x] 3.2 Run the imported project's full documented Python self-check/tests;
  verify representative CLI and adapter outputs from the module root.
- [x] 3.3 Confirm Forge portal/provider DTO compatibility and verify generated
  sibling shims are still recorded as follow-up work, not claimed portable.
- [x] 3.4 Confirm no `.gitmodules`, nested `.git`, untracked origin data,
  omitted tracked source or accidental sibling modification.
- [x] 3.5 Run the Governance fleet audit with an explicit workspace root and
  confirm the four Forge crate declarations resolve without `PROJECT_MISSING`.
- [x] 3.6 Verify a workspace-escaping registry path is refused by the nested
  path handling added to the historical migration tool.

## 4. Verification

- [x] 4.1 Run Workspace Governance documented self-check/test commands from
  `modules/workspace-governance/`; record exact results.
- [x] 4.2 Run the Forge workspace build and source boundary checks to confirm
  the import does not alter Forge caller behavior.
- [x] 4.3 Run `node scripts/check-openspec-change-names.mjs`,
  `openspec validate --all --strict --no-interactive`, and `git diff --check`.
- [x] 4.4 Run `forge gate --dry-run`, then full bounded Forge Gate; record the
  verdict and attribution in HANDOFF before archive.
- [x] 4.5 Review every proposal impact row, archive without `--skip-specs`,
  promote canonical specs, commit related implementation, update the handoff
  pointer, commit HANDOFF only, and stop without pushing.
