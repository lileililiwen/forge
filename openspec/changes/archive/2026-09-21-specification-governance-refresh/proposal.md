# Proposal: Canonical specification and governance refresh

## Why

All 24 promoted specs still contain the generated `Purpose: TBD` placeholder.
The OpenSpec config and foundation ADR also retain planning-only wording even
though the repository now contains implemented code. This weakens traceability
and makes strict validation appear stronger than the actual publication
quality. The roadmap's evidence boundaries need to be current and explicit.

## What Changes

- Replace every canonical spec purpose with an accurate capability purpose.
- Refresh OpenSpec context, ADR historical wording, README, roadmap, coverage
  and HANDOFF claims to distinguish archived implementation evidence, current
  audit follow-ups, simulated fixtures and real runtime evidence.
- Add a traceability check that rejects placeholders, stale active pointers and
  unsupported completion claims.

## BFS Impact Map

- **Capabilities and flows:** OpenSpec authoring, roadmap selection, handoff,
  evidence reporting and completion claims.
- **Modules, contracts and persistence:** canonical specs, `openspec/config`,
  docs/ADR, scripts and no runtime data.
- **Callers:** contributors and delegated implementers using repository docs.
- **Dependencies:** the three implementation/evidence packages above and
  archived handoff records.
- **Failure and boundary behavior:** placeholder docs, stale status, missing
  current pointer, and simulated evidence presented as runtime verification.
- **Tests:** documentation/link/placeholder/traceability scripts and strict
  OpenSpec validation.
- **Compatibility/security/privacy:** documentation-only; do not alter product
  contracts or expose secrets from historical evidence.
- **Unaffected:** runtime behavior, provider protocols and generated projects.

## Capabilities

### New Capabilities

- `specification-governance-refresh`: truthful canonical specs and evidence
  traceability.

### Modified Capabilities

- All promoted capability specifications and repository completion governance.

## Non-goals

This package does not implement runtime fixes, archive any new implementation,
or convert historical evidence into current provider/toolchain proof.
