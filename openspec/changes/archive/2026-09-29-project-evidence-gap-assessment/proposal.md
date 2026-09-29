# Proposal: project evidence gap assessment

## Why

Forge needs to explain why a project is absent from portfolio views or cannot
be acted on. Missing descriptions, tags, CI, Compose files, manifests, and
documentation are different evidence gaps and must not be collapsed into one
health value. Labrys, Hermora, and Hypora need comparable findings even though
their stacks and maturity differ.

## What Changes

- Extend the existing doctor/evidence model with project-catalog findings.
- Detect missing, stale, invalid, unavailable, and unverified metadata for one
  project or an entire catalog.
- Emit stable finding IDs, source provenance, confidence, remediation class,
  and evidence references.
- Support `forge project gaps` and machine-readable JSON/NDJSON output.
- Preserve explicit `PASS`, `WARN`, `FAIL`, `UNAVAILABLE`, and
  `NOT_APPLICABLE` distinctions.

## Package Boundary and Split Assessment

This package only assesses and reports gaps. It does not generate descriptions,
write files, update GitHub, or execute CI. Those changes require separate
approval and rollback boundaries. The package depends on the catalog contract
but remains independently useful as a read-only doctor surface.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `project-evidence-gap-assessment` | Produce evidence-backed project findings | Forge Rust doctor/Core | `forge-project-evidence/0.1.0` | Catalog query contract | Fixture findings and doctor tests |
| `project-local-remediation-plans` | Convert supported findings to local plans | Forge Rust | Remediation plan contract | This package | Plan/diff/conflict tests |
| `project-semantic-description-review` | Route semantic findings to review | Forge Rust/provider boundary | Review proposal contract | This package | Provenance and approval tests |
| `github-project-metadata-adapter` | Observe GitHub evidence | Forge Rust adapter | Provider protocol | Catalog and findings | Mock GitHub responses |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge doctor | `src/doctor/mod.rs` | Existing verdicts, finding IDs, remediation classes | Needs catalog-wide scope and metadata rules | Forge | **Extend shared owner** |
| Specification remediation | `src/spec/mod.rs` | Existing deterministic/semantic/manual routing | It creates bounded specs, not all metadata repairs | Forge | **Adopt** |
| Driftwatchdog | Gate/provider contracts | Quality and runtime evidence | External execution remains provider-owned | Driftwatchdog | **Adapt through a generic adapter** |
| workspace-governance | Governance adapter | Policy declarations and metadata expectations | Must not become a Forge runtime dependency | workspace-governance | **Adapt through a generic adapter** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Findings | New metadata-gap rules and stable remediation classifications |
| Doctor | Single-project and fleet-scoped assessment |
| Query | Findings filterable by project, category, status, and remediation class |
| Persistence | Findings may be timestamped observations; no silent health promotion |
| Failure | Missing source, malformed metadata, stale observations, and unavailable providers are explicit |
| Tests | Success, empty, duplicate, stale, unavailable, and not-applicable fixtures |
| Security | Redact credentials and provider response bodies from evidence |
| Unaffected | File writes, GitHub writes, semantic generation, deployment, and CI execution |

## Capabilities

- `project-evidence-gap-assessment`: evidence-backed metadata and project
  readiness findings.

## Non-goals

- Automatically fixing findings.
- Treating missing metadata as a deployment failure when it is not applicable.
- Replacing Driftwatchdog quality gates or project-native tests.
- Inferring production readiness from repository metadata alone.

Source: requirement.md §7, §23, §32, §34.
