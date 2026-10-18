# Proposal: record the archived governance origin and Forge maintenance owner

## Why

Forge's current handoff, import guide, roadmap and canonical requirement say
that the original Workspace Governance repository must remain available until
consumers migrate. The user has confirmed that origin is already archived and
that any required changes belong in Forge's in-tree copy. The current records
therefore describe the wrong maintenance boundary.

## What Changes

- Record the original repository as archived and
  `modules/workspace-governance/` as the maintained source.
- Make module source-size conformance an in-tree Forge quality task, not an
  archival prerequisite.
- Keep sibling consumer portability as independent Forge migration work.
- Update the existing canonical in-tree ownership requirement.

## Package Boundary and Split Assessment

This is one documentation and canonical-spec correction owned by Forge. It
does not change the imported project implementation, reopen the origin, or
migrate external sibling consumers. The user supplied the archive status;
Forge records that status without performing a remote operation. The single
acceptance oracle is that current Forge docs/specs assign maintenance to the
in-tree copy and no longer make outstanding work block the completed archive.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-governance-origin-archive-status` | Correct Forge's recorded origin and maintenance status | Forge documentation and OpenSpec | `modules/workspace-governance/` is the maintained copy | in-tree import and user-confirmed archive | current docs and canonical spec state the same owner and status |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Original Workspace Governance repository | User-confirmed archived origin; historical import revision in `HANDOFF.md` | Six source-only commits are retained in Forge history | No longer a maintenance target | archived origin; no release boundary for new changes | **keep local** to the Forge-maintained copy |
| Forge imported project | `modules/workspace-governance/`, `openspec/specs/forge-workspace-governance-in-tree/spec.md` | Independently runnable Python project, tests, schemas and history | Older docs still direct work to an unarchived origin | Forge owns the maintained monorepo copy | **extend shared owner** by correcting status and maintenance records |

## User Experience and Interface Impact

UI/UX: N/A. This changes maintainer documentation and the maintenance
ownership record; it adds no product workflow, screen, or interaction.

## BFS Impact Map

| Area | Impact and invariant |
|---|---|
| Maintenance ownership | Forge's in-tree project is the only active source for needed changes |
| Origin status | The original repository is recorded as archived, based on the user's confirmation |
| Source conformance | Remains a Forge quality task and is not an archive gate |
| Consumers | Sibling portability remains queued; it does not reopen the origin |
| Runtime and contracts | No executable behavior or project data changes |
| Verification | Strict OpenSpec validation, targeted wording audit, diff checks and Forge Gate |

## Capabilities

- `forge-workspace-governance-in-tree`

## Non-goals

- Do not modify or unarchive the original repository.
- Do not edit `modules/workspace-governance/` source files in this status-only
  change.
- Do not complete source-size splitting or sibling consumer migration here.
- Do not perform any remote operation.
