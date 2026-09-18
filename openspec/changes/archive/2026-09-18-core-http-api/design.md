## Context

Optional HTTP transport over Core is planned from [requirement.md](../../../requirement.md) §35. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [mature-mcp-surface](../mature-mcp-surface/proposal.md), [adapter-deployment](../adapter-deployment/proposal.md), [external-planes-analytics](../external-planes-analytics/proposal.md)

## Goals / Non-Goals

**Goals:** Equivalent HTTP domain behavior; Authorized asynchronous operations. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Duplicating Core logic, requiring HTTP in v0.1, or assuming an unauthenticated public listener is acceptable.

## Decisions

Expose versioned project list/create/inspect plus project doctor/features/upgrades/specs/agents/deployments routes over Core. Use operation IDs for long-running work and explicit idempotency and concurrency preconditions. Authenticate callers and authorize each project/action; CLI stays usable without an API server and MCP may still call Core directly.

### Contracts and ownership

Affected surfaces: API routing, Core contracts, operation resources and access control. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Equivalent HTTP domain behavior:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Authorized asynchronous operations:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Binding HTTP broadens access beyond a local process; default network exposure and authentication mode must be resolved before enabling a listener. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Binding HTTP broadens access beyond a local process; default network exposure and authentication mode must be resolved before enabling a listener.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across api routing, core contracts, operation resources and access control. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
