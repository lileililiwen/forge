## Context

Gated resumable releases and publication is planned from [requirement.md](../../../requirement.md) §29, §34, §43. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [repository-distribution](../repository-distribution/proposal.md), [documentation-translation](../documentation-translation/proposal.md), [quality-policy-integration](../quality-policy-integration/proposal.md)

## Goals / Non-Goals

**Goals:** Verified release preparation; Resumable multi-destination publication. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Publishing during docs bootstrap, force replacing tags, or treating partial publication as a complete release.

## Decisions

Capture a release plan with source revision, semver, checks, changelog, optional translations and selected destinations. Enforce doctor/tests/DriftWatch before version, commit, tag, push and package/container publication. Journal external stages with immutable release identity and explicit authorization. Translation stage is conditional; no configured locale means no translation dependency at runtime. forge release orchestrates preparation and execution; forge publish selects publication stages from an already verified release identity and cannot bypass its checks.

### Contracts and ownership

Affected surfaces: Release state machine, version/changelog, Git distribution, package/container providers. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Verified release preparation:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Resumable multi-destination publication:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Published artifacts and pushed tags may be irreversible; resumable stages need idempotent provider operations, not fictional rollback. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Published artifacts and pushed tags may be irreversible; resumable stages need idempotent provider operations, not fictional rollback.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across release state machine, version/changelog, git distribution, package/container providers. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
