# Proposal: Repository Discoverability Governance

## Why

Workspace discovery currently identifies unregistered projects and validates
basic declarations, but it cannot tell whether a project has a license, a
complete discoverability description, or matching GitHub metadata. The
init-docs workflow can describe those requirements, but projects cannot be
audited or repaired consistently by the shared Python tooling.

## What Changes

- Extend `.project.json` with source-backed `discoverability` metadata and
  separate `publication` evidence.
- Make standard adopted profiles require `LICENSE` and complete local
  discoverability metadata; retain the inventory profile exception.
- Add stable audit findings for missing/invalid local metadata and unverified
  publication.
- Extend `init_project.py` with additive, explicit local repair.
- Add an authenticated GitHub CLI adapter for read-only checking and explicit
  metadata publication/remote creation.

## Package Boundary and Split Assessment

This is one package because all included surfaces share the same owner
(`workspace-governance`), declaration lifecycle, Python runtime, and acceptance
boundary: a discovered project can be classified as locally standardized and
then independently verified against GitHub. The package is not split because
the GitHub adapter consumes the exact discoverability contract introduced by
the local audit; separating them would create an unusable intermediate
metadata shape. Deployment execution, repository visibility policy decisions,
and product-specific SEO content remain outside this package.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `workspace-governance` | `scripts/workspace_check.py`, `schemas/project.schema.json`, `scripts/init_project.py` | Existing registry discovery, declaration validation, additive bootstrap, JSON findings | Needs discoverability/publication vocabulary and `gh` subprocess boundary | Workspace governance owns policy/audit/bootstrap; releases with this repository | **extend shared owner** |
| `driftwatchdog` | Existing gate/runtime boundary referenced by `.project.json` | Evidence execution and quality-gate aggregation | Must not own repository identity, GitHub mutation, or documentation policy | Driftwatchdog remains a consumer/runtime | **adapt through a generic adapter** |
| `forge` | `scripts/forge_governance_adapter.py` | Read-only governance observation contract | Consumes findings; must not become repository mutation owner | Forge remains portfolio consumer | **adapt through a generic adapter** |
| `common`/`manager` siblings | No matching shared repository/package in `/home/paul/code` | No reusable GitHub metadata ownership found | Creating one would add a second authority for workspace policy | No new manager project justified | **keep local** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Actors/flows | Workspace auditors, init-docs agents, project owners, authenticated GitHub operators |
| Contracts/data | `.project.json` schema, profile requirements, stable finding codes, publication evidence |
| Local files | `LICENSE`, `.project.json`, README-derived metadata; existing content preserved |
| Integrations | `gh auth status`, `gh repo view`, `gh repo edit`, `gh repo create`, Git remote inspection |
| Failure boundaries | Missing auth, missing origin, ambiguous owner/visibility, malformed metadata, GitHub drift, partial publication |
| Tests | Unit tests with temporary repositories and injected subprocess runners; no live mutation required |
| Unaffected | Product source, deployment execution, Forge protocol shape, Driftwatchdog gate semantics, existing registry paths |

## Capabilities

- `local-discoverability-audit`
- `local-metadata-repair`
- `github-repository-metadata-verification`

## Non-goals

- Automatically inventing claims, audience, SEO phrases, or product capabilities.
- Storing or managing GitHub tokens.
- Changing repository visibility without an explicit operator choice.
- Force-pushing, deleting topics, rewriting remotes, or deploying projects.
- Replacing product-owned README or license content wholesale.
