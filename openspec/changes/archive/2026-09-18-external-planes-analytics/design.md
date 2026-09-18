## Context

Existing content and analytics planes with project metrics is planned from [requirement.md](../../../requirement.md) §32, §33. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [agent-runtime-workflows](../agent-runtime-workflows/proposal.md), [adapter-deployment](../adapter-deployment/proposal.md), [repository-distribution](../repository-distribution/proposal.md)

## Goals / Non-Goals

**Goals:** External system identity and health; Timestamped project metrics. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Rewriting analytics/content services, inventing production telemetry, or executing remote writes during metric retrieval.

## Decisions

Reuse existing GitHub analytics and unified content systems through adapters. Join observations via explicit project/provider identities and timestamps; present counts for projects, running agents, queued specs, quality states, deployment states, stars and seven-day growth. Keep content ownership external; Forge records references and health, not a duplicate CMS.

### Contracts and ownership

Affected surfaces: Integrations, project ID mapping, analytics snapshots and external content references. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — External system identity and health:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Timestamped project metrics:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Metrics may have different windows and unavailable providers; do not sum unknown values into authoritative totals. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Metrics may have different windows and unavailable providers; do not sum unknown values into authoritative totals.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across integrations, project id mapping, analytics snapshots and external content references. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
