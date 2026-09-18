## Context

Portable AI procedures over stable operations is planned from [requirement.md](../../../requirement.md) §19, §32. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [mature-mcp-surface](../mature-mcp-surface/proposal.md), [validated-intent-planner](../validated-intent-planner/proposal.md), [adapter-deployment](../adapter-deployment/proposal.md)

## Goals / Non-Goals

**Goals:** Discoverable operation procedures; Procedures do not bypass Core. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Another agent runtime, duplicated generators inside prompts, or binding procedures to one model or IDE.

## Decisions

Provide create-project, upgrade-project, prepare-release, fix-quality-findings, onboard-existing-project, deploy-project, mirror-repository and translate-docs SOPs. Procedures discover capabilities, prefer certified assets, use Core operations, run doctor/tests/quality and report gaps. Store no platform implementation or provider-specific authority bypass in procedures.

### Contracts and ownership

Affected surfaces: Versioned procedure resources, CLI/MCP references and procedure validation. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Discoverable operation procedures:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Procedures do not bypass Core:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Agent prompt compliance is not an enforcement boundary; Core still validates every operation. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Agent prompt compliance is not an enforcement boundary; Core still validates every operation.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across versioned procedure resources, cli/mcp references and procedure validation. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
