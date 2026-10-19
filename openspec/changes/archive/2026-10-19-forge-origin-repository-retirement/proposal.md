# Proposal: Retire migrated origin repositories

## Why

Forge now owns the migrated DriftWatchdog and cargo-* tools, Workspace
Governance, and the .NET and Rust platform-library source trees. The Forge
checkout carries source-import verifiers and consumer workflows, but GitHub
still has active origin repositories for some migrated projects. Leaving those
origins writable keeps an ambiguous second maintenance path and contradicts
the consolidation boundary. The retirement must be based on current remote
state and verified Forge provenance, and it must preserve local-only user work.

## What Changes

- Reconcile the exact migrated origin set against Forge source imports,
  preserved source history, project consumers and current GitHub archive state.
- Migrate active Forge-owned source, project-homepage and maintainer directions
  and internal project-registry/provider-preset paths to the corresponding
  paths in Forge. Preserve links needed to retrieve already-published
  historical release assets and label them as such.
- Archive only existing, active GitHub origin repositories for migrated
  projects. Treat already archived repositories as complete and confirmed
  missing repositories as absent; never create, delete, transfer or rename a
  repository.
- Verify each successful remote archive through GitHub and record the
  repository-by-repository result and provenance in HANDOFF.
- Preserve the local sibling checkouts and all pre-existing untracked files;
  repository archival is a remote setting, not local checkout cleanup.
- Keep Forge-owned build, test, Gate and publication workflows functional
  after origin retirement.

## BFS Impact Map

| Surface | Evidence | Change / verification |
|---|---|---|
| Imported source | `modules/*`, `crates/*`, and the three deterministic import checkers | Run all applicable import checkers and confirm source-only commits remain in Forge history. |
| Consumers | Root Cargo workspace, module Gate declarations, `.github/workflows/`, Governance module registrations and import scripts | Search for external checkout/path dependencies; build and run the repository verification suite. |
| Remote repository state | GitHub API (`gh repo view` / `gh api`) for the exact migrated origins | Re-read each state immediately before action; archive only an exact, existing, active target; re-read and require `archived=true`. |
| Local sibling checkouts | `/home/paul/code/workspace-governance`, `/home/paul/code/dotnet-platform-libs`, `/home/paul/code/rust-platform-libs` | Capture Git status before and after; do not edit, reset, clean or delete them. Preserve their existing untracked files. |
| Unrelated repositories | `platform-contracts`, `alltools-platform`, `gitrunbykaru`, and consumer products | Keep out of scope; verify archive targets match only the migrated origin allowlist. |
| Maintainer and project metadata | Cargo/npm metadata, Forge module declarations, taxonomy entries, copied-source provenance and clone instructions | Point active source/maintenance links at Forge; retain historical release links only where existing artifacts remain part of a supported compatibility path. |
| Governance and evidence | `ROADMAP.md`, `HANDOFF.md`, archived import packages and OpenSpec canonical specs | Record exact archived/already-archived/absent/error results, commit identities, check commands and any unresolved remote state. |

## Capabilities

- `forge-origin-repository-retirement`: retire writable remote origins after
  the corresponding source and consumers are verified as Forge-owned.

## Non-goals

- Deleting, transferring, renaming or rewriting remote repositories or their
  history.
- Removing local sibling checkouts, cleaning their worktrees, or changing
  their untracked/user-owned files.
- Archiving `platform-contracts`, `alltools-platform`, `gitrunbykaru`, or any
  consumer/product repository.
- Changing package identities, crate publication metadata, release workflows,
  Forge source imports, or project-specific checks.
- Pushing Forge changes or publishing packages.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Independent oracle |
|---|---|---|---|---|
| `forge-origin-repository-retirement` | Every existing migrated source origin is read-only while Forge imports and consumers remain verified | Forge Rust control plane; remote GitHub repository settings | Exact origin allowlist and GitHub archive state; no source or consumer rewrite | Import verifiers, history ancestry checks, consumer search/build/tests, and GitHub `archived` state |

The action is one final consolidation gate over a fixed set of migrated
origins. It does not combine unrelated product repositories or package
publication. Current GitHub state is a baseline only; implementation must
re-query and decide from live state before each operation.
