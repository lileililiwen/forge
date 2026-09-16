## Context

Versioned project model and CLI foundation is planned from [requirement.md](../../../requirement.md) §1, §2, §3, §4, §5, §6, §7, §34, §37, §38, §39, §44, §45, §47. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: None; this establishes the foundation.

## Goals / Non-Goals

**Goals:** Versioned manifest contract; Registry and CLI identity; Independent command contract. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Runtime infrastructure, profiles, generation, HTTP, MCP, or a requirement for generated projects to depend on Forge.

## Decisions

Use a modular monolith with typed, language-neutral Core contracts. Plan Rust CLI/Core and SQLite as the recommended baseline from section 37; record a foundation ADR before implementation. forge.yaml is canonical; accept platform.yaml only as an explicit legacy import source, rejecting ambiguous dual manifests. Registry records reference project IDs and canonical paths; observations carry timestamps and unknown states. Mutations use schema validation and atomic manifest writes with transactional registry updates and recoverable reconciliation.

### Contracts and ownership

Affected surfaces: Core, Registry, CLI; manifest schema and local registry; future adapters and transports. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Versioned manifest contract:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Registry and CLI identity:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R3 — Independent command contract:** implement the observable outcomes in DFS task 2.3; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Database and manifest writes cannot share one transaction; journal reconciliation and never report a partial write as successful. Unsupported schema versions must not be rewritten. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Database and manifest writes cannot share one transaction; journal reconciliation and never report a partial write as successful. Unsupported schema versions must not be rewritten.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across core, registry, cli; manifest schema and local registry; future adapters and transports. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
