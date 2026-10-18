# Design: forge-workspace-governance-in-tree

## Implementation boundary

- Repository: `/home/paul/code/forge`, on `main`.
- Imported project root: `modules/workspace-governance/`, retaining all
  tracked source-relative paths.
- Import all tracked project content, including AGENTS/HANDOFF/ROADMAP,
  OpenSpec active and archived packages, tests, scripts, schemas, profiles,
  templates, docs, and registry data. Do not filter using ignore rules.
- Apply a narrow monorepo overlay to `projects.json`: retain the IDs
  `driftwatchdog`, `cargo-audit-boost`, `cargo-matrix-boost`, and `cargo-sonar`
  while changing their paths to `forge/crates/<id>`. Record old/new paths in
  `migration/forge-monorepo-path-reconciliation.json`; each destination must
  contain the project's declaration.
- Adapt fleet-level tests to resolve the workspace root from `WORKSPACE_ROOT`
  or the ordinary Forge `modules/workspace-governance` layout. The environment
  override wins; the standalone sibling layout keeps its parent-root default.
- Preserve source Git ancestry in Forge's Git object graph and make the six
  local-only commits ancestors of the import commit. Do not add a `.gitmodules`
  entry or nested `.git` directory.
- Update only import provenance, consolidation roadmap, relevant README/architecture
  text, OpenSpec artifacts and HANDOFF. Do not edit the sibling checkout.

## Language and runtime

Workspace Governance remains Python 3.11 with its own existing dependency
manifest, scripts, tests and documented self-check. Forge caller remains Rust
stable in the root Cargo workspace. The imported Python project is not added
as a Cargo member and is not built into generated projects. Run Python checks
from `modules/workspace-governance/` exactly as its README/AGENTS specify;
run Forge's provider-contract tests and build from the Forge root.

## Ownership and shared code

The imported directory is the maintained Workspace Governance project.
Forge owns the human-facing CLI/dashboard and its adapter, not duplicated
policy implementations. Python scripts keep module-root-relative path
behavior. Existing adapter resolution remains unchanged in this source import
package; Forge call-site migration follows as a separately verified package.
Generated sibling projects must not invoke this in-tree source path; their
portable transition distributes versioned Forge-owned assets or uses Forge
commands.

## User experience and interface

No new user workflow. Existing CLI `forge governance list|status|use` and
dashboard governance observations remain the entry points and are unchanged
in this source import package. The follow-up consumer migration owns adapter
resolution and its CLI/provider/browser compatibility checks.

## Behavioral model

| Input | Resolution | Outcome |
|---|---|---|
| Source import only | Existing Forge resolution and invocation | No adapter behavior change in this package |
| Imported module check | Execute documented commands from module root | Do not rely on sibling checkout layout |
| Missing source checkout, conflict or history mismatch | Import precondition fails | Stop without claiming complete migration |

The import itself must be content-complete: all source tracked paths remain
present and unchanged except the registry remap, nested-path/test-root
adaptations and 15 whitespace-only normalizations named in the import
verifier; two documented monorepo support files are added. All six source-only
commits are ancestors reachable from the Forge commit containing the imported
project. Re-running the parity check is deterministic.

## Contract and compatibility

- No provider contract version or JSON field changes.
- No Forge database migration.
- No source files deleted from the sibling.
- No `.gitmodules` or runtime Git checkout dependency.
- Module's own workspace path remains its root; Forge tests should invoke it
  using an explicit absolute or repository-relative path.
- The source remote remains an origin-history reference only; no remote write
  is performed.

## Failure and recovery

- Missing source checkout or missing source commit: stop before import; retain
  the active OpenSpec and report exact source prerequisite.
- Existing destination: refuse overwrite; compare and reconcile explicitly.
- Path/blob mismatch: fail the parity oracle and do not mark import complete.
- Missing runtime module in a packaged Forge binary: typed unavailable; no
  sibling path fallback. Packaging the module for distribution is a later
  separately verified decision.
- Existing Forge adapter contract regressions: block completion.
- Interrupted import: because the module is tracked content in one commit,
  discard only staged import paths and retry; never remove unrelated files.

## Verification oracle

1. Source-history assertion: each of the six source-only commit IDs is an
   ancestor of the Forge import commit.
2. Exact source-path set equality against source `git ls-tree` at
   `2c766c64...`; compare each blob/mode except the registry, nested-path and
   test-root adaptations. Verify whitespace-only edits preserve normalized
   text. Reject undeclared paths or content drift.
3. Run Workspace Governance's documented self-check and test command from
   the imported root under Python 3.11; record exact counts and failures.
4. Confirm Forge source and provider behavior are unchanged by the import; run
   the Rust workspace build and source boundary checks.
5. Run OpenSpec name preflight, strict validation, `git diff --check`, Gate
   dry-run and full bounded Gate; record verdicts before archive.

No browser change is planned. Retain any existing portal governance contract
tests; they establish transport compatibility but do not substitute for
Python self-check or Gate.

## Decision ledger

- **Resolved:** destination is `modules/workspace-governance/` to identify an
  ordinary project root without colliding with Forge source or legacy data.
- **Resolved:** preserve source Git ancestry by importing commits, rather than
  copying a snapshot; the six commits have active governance content.
- **Resolved:** `platform-contracts` remains separate because its wire-schema
  lifecycle and consumers are independent.
- **Resolved:** the four moved tooling projects retain their IDs while their
  paths move from removed sibling directories to `forge/crates/<id>`; a
  migration record and live audit test protect this change.
- **Deferred:** exact binary distribution of the Python module and migration
  of external consumers belong to the dependency-ordered follow-ups.
- **Deferred:** origin remote archival occurs only after consumer-portability
  evidence; no remote write in this package.
