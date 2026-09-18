## Context

Optional portal for the shared control plane is planned from [requirement.md](../../../requirement.md) §36. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [core-http-api](../core-http-api/proposal.md), [external-planes-analytics](../external-planes-analytics/proposal.md), [semantic-ui-patterns](../semantic-ui-patterns/proposal.md)

## Goals / Non-Goals

**Goals:** Shared project and lifecycle views; Accessible controlled operations. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** A portal in the early MVP, another backend policy engine, or making graphical access mandatory.

## Decisions

Consume Core HTTP APIs for dashboard/projects/features/components/policies/specs/agents/deployments/repositories/documentation/analytics/servers/settings. Show provenance, current/target maturity and operation progress; actions invoke the same validated operations. Choose portal framework in a later ADR; ASP.NET Core or Next.js are possibilities, not requirements.

### Contracts and ownership

Affected surfaces: Portal navigation, API client, accessible operation views and project status. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Shared project and lifecycle views:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Accessible controlled operations:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

A dashboard can imply stronger health than evidence supports; display unknown/stale and partial results prominently. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- A dashboard can imply stronger health than evidence supports; display unknown/stale and partial results prominently.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across portal navigation, api client, accessible operation views and project status. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
