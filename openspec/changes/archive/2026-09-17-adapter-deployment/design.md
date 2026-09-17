## Context

Simple deployment targets and observed runtime state is planned from [requirement.md](../../../requirement.md) §30, §34, §43. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [release-publishing](../release-publishing/proposal.md)

## Goals / Non-Goals

**Goals:** Explicit adapter deployment; Observed health and recovery. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Kubernetes control plane, arbitrary shell from natural language, or distributed scheduling.

## Decisions

Start with Docker Compose and SSH/local-host targets suitable for home servers and VPS. Resolve explicit project/target/artifact, validate configuration and present actions. Bound execution and verify target health before reporting running. Record previous artifact and recovery procedure where supported.

### Contracts and ownership

Affected surfaces: Deployment adapters, artifact selection, registry observations, target and secret references. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Explicit adapter deployment:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Observed health and recovery:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Remote hosts are independent failure domains; disconnected means unknown, not offline proof. Recovery may require operator action. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Remote hosts are independent failure domains; disconnected means unknown, not offline proof. Recovery may require operator action.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across deployment adapters, artifact selection, registry observations, target and secret references. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
