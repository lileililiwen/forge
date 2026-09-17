## Context

Findings to deterministic fixes or bounded specs is planned from [requirement.md](../../../requirement.md) §22, §26, §32, §34, §41. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [project-upgrade-orchestration](../project-upgrade-orchestration/proposal.md), [quality-policy-integration](../quality-policy-integration/proposal.md)

## Goals / Non-Goals

**Goals:** Traceable spec generation; Remediation routing. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Replacing OpenSpec, generating unbounded rewrites, or running agents simply because a spec was generated.

## Decisions

Normalize findings into deterministic, semantic and manual queues. Deterministic fixes require a supported migration and validation; semantic changes become bounded OpenSpec packages using the existing proposal workflow. Store provenance from finding IDs and project revision to generated spec IDs; execution is routed to the later agent adapter, never implicit on generation.

### Contracts and ownership

Affected surfaces: Spec integration, doctor findings, upgrade conflicts and existing Codex proposal workflow. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Traceable spec generation:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Remediation routing:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Overlapping findings can create duplicate or contradictory work; merge only compatible scopes and preserve source evidence. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Overlapping findings can create duplicate or contradictory work; merge only compatible scopes and preserve source evidence.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across spec integration, doctor findings, upgrade conflicts and existing codex proposal workflow. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
