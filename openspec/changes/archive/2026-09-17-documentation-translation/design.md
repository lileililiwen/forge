## Context

Derivative documentation and source-hash freshness is planned from [requirement.md](../../../requirement.md) §28, §34, §43. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [core-manifest-registry](../core-manifest-registry/proposal.md)

## Goals / Non-Goals

**Goals:** Canonical and derivative documentation; Incremental and reliable translation. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Translation enabled by default, overwriting canonical docs, or claiming translation quality from provider success alone.

## Decisions

Keep the configured source language/document canonical. Track source hashes and segment hashes for incremental translation; require a configured provider and keep outputs in explicit derivative paths. Generate to staging and mark current only after the source hash is rechecked and output validated.

### Contracts and ownership

Affected surfaces: Documentation metadata, translation provider adapter, CLI and doctor freshness. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Canonical and derivative documentation:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Incremental and reliable translation:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

AI translation can alter commands or links; preserve fenced code, destinations and explicit non-translatable terms and expose review state. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- AI translation can alter commands or links; preserve fenced code, destinations and explicit non-translatable terms and expose review state.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across documentation metadata, translation provider adapter, cli and doctor freshness. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
