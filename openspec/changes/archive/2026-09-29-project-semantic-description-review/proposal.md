# Proposal: semantic project description and classification review

## Why

Some project metadata cannot be repaired deterministically. A useful
description, domain classification, or portfolio tag may require interpreting
README content, source structure, OpenSpec capabilities, and current evidence.
Forge should propose such changes with provenance and human approval instead of
silently presenting generated text as fact.

## What Changes

- Define reviewable semantic proposals for descriptions, domains, portfolio
  tags, profiles, and lifecycle classifications.
- Record current value, suggested value, evidence sources, confidence,
  generator/provider, and approval state.
- Add `forge describe suggest`, `forge classify suggest`, and review/approve
  operations over the shared remediation model.
- Route unresolved or conflicting interpretations to bounded OpenSpec work or
  manual review.
- Keep generated suggestions distinct from approved project declarations and
  observed evidence.

## Package Boundary and Split Assessment

This package owns proposal and approval state for semantic metadata. It does
not own deterministic file patches, GitHub API transport, or agent execution.
Those are separate consumers of the review contract.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `project-semantic-description-review` | Approve or reject traceable semantic metadata proposals | Forge Rust Core/CLI plus optional provider | Review proposal contract | Evidence gaps, remediation plan | Approval, rejection, provenance, and conflict tests |
| `project-local-remediation-plans` | Apply approved local results | Forge Rust | Remediation plan contract | This package | Approved proposal to local diff tests |
| `github-project-metadata-adapter` | Apply approved GitHub results | Forge Rust adapter | Provider contract | This package | Approved proposal to PR tests |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Specification remediation | `src/spec/mod.rs` | Semantic/manual routing and bounded proposal generation | Needs metadata-specific approval records | Forge | **Extend shared owner** |
| Agent runtime | `src/agent/` | Controlled agent sessions and project scoping | Suggestions must remain reviewable and bounded | Forge | **Adapt through a generic adapter** |
| Analytics/portfolio | `src/analytics/`, `src/portfolio/` | Timestamped project observations and user-owned metadata | Semantic metadata is not interest or billing data | Forge | **Keep local** |
| GitHub provider | Future adapter boundary | External metadata authority | Authentication and rate limits are provider-specific | Forge | **Defer to adapter package** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Proposal state | `suggested`, `approved`, `rejected`, `superseded`, `conflicted` |
| Evidence | Source paths, revisions, excerpts or hashes, confidence, provider identity |
| Approval | Explicit operator decision before any local or external write |
| Failure | Conflicting evidence, stale revision, unavailable model/provider, malformed suggestion |
| Persistence | Review records are project-scoped and revision-bound |
| Security | Redact secrets and prevent prompt/provider output from becoming authority automatically |
| Unaffected | Deterministic CI/Compose templates, GitHub transport, deployment, billing |

## Capabilities

- `project-semantic-description-review`: traceable human review for semantic
  project descriptions and classifications.

## Non-goals

- Auto-publishing AI-generated descriptions.
- Treating confidence as truth or deployment evidence.
- Running unconstrained agents or editing arbitrary project files.
- Replacing human ownership of product identity and lifecycle decisions.

Source: requirement.md §21, §22, §32, §34.
