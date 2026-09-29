# Proposal: project-local remediation plans

## Why

Once Forge can identify missing metadata and runtime contracts, it should help
repair deterministic gaps without forcing each project to invent its own
workflow. Repairs must be previewable, ownership-aware, reversible, and
separate from semantic judgment or external GitHub writes.

## What Changes

- Define `forge-remediation-plan/0.1.0` for finding-to-action plans, diffs,
  ownership receipts, preconditions, and outcomes.
- Add `forge remediate scan|plan|diff|apply` for local project targets and
  catalog-selected targets.
- Consume the existing standard-pack registry for versioned CI and Compose
  assets rather than introducing a second template system.
- Support safe installation of the selected standard pack's project-local CI
  snapshot for automatic missing-CI findings; the pack renderer supplies CI
  and companion `.standard/` files together with their ownership receipt.
- Keep metadata editing, `.project.json` changes, and documentation-link repair
  out of this package until their ownership and merge contracts are specified.
- Require explicit confirmation, refuse unowned collisions, and record
  rollback information for every applied action.

## Package Boundary and Split Assessment

This package owns local deterministic/template remediation. Semantic
description generation and GitHub mutation are separate packages because they
have different approval, provider, and failure lifecycles.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `project-local-remediation-plans` | Preview and apply safe local repairs | Forge Rust Core/CLI | `forge-remediation-plan/0.1.0` | Evidence gaps, standard packs | File-diff, collision, rollback, idempotency tests |
| `project-semantic-description-review` | Review semantic metadata proposals | Forge Rust/provider boundary | Review proposal contract | Evidence gaps | Approval and no-auto-apply tests |
| `github-project-metadata-adapter` | Apply approved external changes | Forge Rust adapter | GitHub provider contract | Remediation plan | Mock provider and PR-mode tests |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Standard packs | `src/standard/` | Versioned owned assets, receipts, diff, upgrade semantics | Must add project metadata consumers without changing pack ownership | Forge | **Extend shared owner** |
| Specification remediation | `src/spec/mod.rs` | Finding routing and bounded proposals | Local deterministic patches need file ownership and rollback | Forge | **Adapt through a generic adapter** |
| Project generation | `src/generate/` | Staging, collision, and cleanup behavior | Generation is creation-oriented, not existing-project repair | Forge | **Adopt** |
| workspace-governance | Metadata declarations | Policy and ownership receipts | Forge must not write sibling policy files implicitly | workspace-governance | **Adapt through a generic adapter** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Plan | Versioned actions, preconditions, file ownership, diff, and outcome |
| Apply | Explicit confirmation, atomic writes where possible, rollback on failure |
| Templates | CI/Compose assets selected by profile and runtime, never guessed silently |
| Registry | Journal remediation intent and result without replacing project source |
| Failure | Conflict, stale revision, missing asset, invalid profile, partial write, and rollback failure |
| Callers | CLI first; later MCP/API consume the same Core plan/apply service |
| Security | No secret generation, secret disclosure, implicit push, or remote mutation |
| Unaffected | Semantic description approval, GitHub API writes, deployment execution |

## Capabilities

- `project-local-remediation-plans`: deterministic and template-based local
  repair planning and application.

## Non-goals

- Blindly overwriting project-owned files.
- Creating fake CI or Compose configurations without a selected contract.
- Running builds, tests, or deployments as a substitute for evidence.
- Pushing commits, tags, or pull requests to GitHub.

Source: requirement.md §14, §22, §26, §32, §41.
