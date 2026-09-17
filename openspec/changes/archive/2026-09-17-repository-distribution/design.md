## Context

Canonical GitHub repository and one-way Gitee mirrors is planned from [requirement.md](../../../requirement.md) §27, §32, §34, §43. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [agent-runtime-workflows](../agent-runtime-workflows/proposal.md)

## Goals / Non-Goals

**Goals:** Canonical one-way distribution; Partial failure and credentials. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Bidirectional synchronization, implicit force pushes, or storing access tokens in manifests.

## Decisions

Keep one explicitly configured primary and provider-qualified mirrors. First support GitHub primary and Gitee mirror through adapters, leaving GitLab/Codeberg future. Expose forge mirror through the shared distribution operation. Validate repository identity and ref policy, dry-run pushes and capture per-remote results. Divergence blocks destructive updates; credentials are externally referenced and redacted.

### Contracts and ownership

Affected surfaces: Distribution, Git adapters, credential references and remote operation records. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Canonical one-way distribution:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Partial failure and credentials:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

A primary push and mirror push are not atomic; record primary success separately and allow safe mirror retry. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- A primary push and mirror push are not atomic; record primary success separately and allow safe mirror retry.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across distribution, git adapters, credential references and remote operation records. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
