## Context

Versioned feature add, remove and upgrade is planned from [requirement.md](../../../requirement.md) §10, §14, §15, §34, §40. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [doctor-maturity-assessment](../doctor-maturity-assessment/proposal.md)

## Goals / Non-Goals

**Goals:** Feature resolution and discovery; Safe lifecycle operations. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Claiming every listed feature works on every stack; bespoke auth generation when a verified supported asset exists.

## Decisions

Descriptors define ID, version, compatibility, dependencies, conflicts, install/upgrade strategy, validation, docs and tests. Catalog the listed auth/admin/postgres/redis/email/storage/audit/telemetry/health-check/rate-limit/background-jobs/search/billing/notifications/i18n/privacy/content/analytics semantics; publish support per profile only with tested assets. Pin the dependency graph and inventory owned changes before applying package-plus-generator or codemod strategies.

### Contracts and ownership

Affected surfaces: Features registry, resolver, generator, CLI, manifests and adapter validation. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Feature resolution and discovery:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Safe lifecycle operations:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Shared file edits and reverse dependencies make removal hazardous; preserve user edits and stop on ownership conflicts. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Shared file edits and reverse dependencies make removal hazardous; preserve user edits and stop on ownership conflicts.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across features registry, resolver, generator, cli, manifests and adapter validation. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
