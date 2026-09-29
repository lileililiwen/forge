# Proposal: GitHub project metadata adapter

## Why

GitHub can help locate and classify repositories and can provide useful
metadata such as topics, descriptions, languages, workflows, releases, and
repository state. It is an external observation and mutation boundary, not the
authority for local project evidence or Forge deployability.

## What Changes

- Define a versioned GitHub metadata adapter for repository observation.
- Read repository descriptions, topics, languages, default branch, archived
  state, workflows, releases, tags, and custom properties when available.
- Normalize GitHub values into the catalog while retaining source and freshness.
- Add rate-limit, authentication, permission, pagination, and unavailable
  provider handling.
- Support approved metadata changes through pull-request mode first; direct
  mutation requires an explicit separate mode and confirmation.
- Keep release tags, GitHub topics, and Forge portfolio tags as separate
  namespaces.

## Package Boundary and Split Assessment

This package owns GitHub transport and provider semantics. Catalog normalization
and remediation plan semantics remain in their owning packages. The adapter
must be replaceable by another Git host without changing Forge Core contracts.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `github-project-metadata-adapter` | Observe and review GitHub metadata | Forge Rust adapter | `forge-github-metadata/0.1.0` | Catalog, evidence, remediation contracts | Mock GitHub API and no-credential tests |
| `project-catalog-query-contract` | Consume normalized observations | Forge Rust Core | Catalog contract | Existing inventory | Catalog parity tests |
| `project-local-remediation-plans` | Consume approved local changes | Forge Rust | Remediation plan contract | Evidence and standard packs | No-remote-write tests |
| `project-semantic-description-review` | Approve semantic proposals | Forge Rust | Review contract | Evidence gaps | Approval state tests |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge provider system | `src/governance.rs`, provider modules | Bounded executable/provider selection and redaction | GitHub needs API auth, pagination, and provider-specific errors | Forge | **Extend shared owner** |
| Release publishing | `src/release/` | Explicit confirmation for tags and publication stages | Metadata observation is not release publication | Forge | **Adapt through a generic adapter** |
| workspace-governance | Governance adapter | Optional portfolio source and policy | Must not require a fixed workspace or sibling checkout | workspace-governance | **Adapt through a generic adapter** |
| GitHub | External API | Repository metadata and PR authority | Network/account/rate limits are unavailable states | External provider | **Adapt through a generic adapter** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Observation | Catalog receives GitHub values with source, revision, and freshness |
| Query | GitHub topics, languages, workflow state, and repository status become filterable |
| Mutation | PR mode creates reviewable changes; direct mode is explicit and gated |
| Configuration | Token reference, host URL, repository identity, and rate limits |
| Failure | Unauthorized, forbidden, not found, rate limited, network unavailable, stale, and partial response |
| Security | Tokens never enter reports; response bodies and secrets are redacted |
| Compatibility | GitHub is optional; local-only Forge remains fully functional |
| Unaffected | Local project source, deployment, CI execution, and Forge deployability verdicts |

## Capabilities

- `github-project-metadata-adapter`: optional GitHub observation and approved
  metadata-change transport.

## Non-goals

- Making GitHub mandatory for catalog discovery.
- Treating GitHub language statistics as build evidence.
- Confusing GitHub release tags with Forge classification tags.
- Pushing directly or changing repository settings implicitly.

Source: requirement.md §7, §32, §33, §38.
