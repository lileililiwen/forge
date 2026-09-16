## Context

Versioned profiles for the five MVP stacks is planned from [requirement.md](../../../requirement.md) §3, §9, §37, §39. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [core-manifest-registry](../core-manifest-registry/proposal.md)

## Goals / Non-Goals

**Goals:** MVP profile discovery; Stack compatibility contract. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** react-web and specialized profiles are later; no feature installer or universal cross-stack implementation.

## Decisions

Define profile descriptors independently of template rendering. Initial IDs are aspnet-web, rust-web, nextjs-web, flutter-app and python-service. Each pins versioned assets and declares capability support, packages, layout, conventions, build/test commands, deployment defaults and policies. Resolve versions explicitly, with no implicit latest downloads.

### Contracts and ownership

Affected surfaces: Profiles, stack adapter contracts, CLI discovery; versioned profile metadata. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — MVP profile discovery:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Stack compatibility contract:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Profile metadata alone does not establish working templates; generation validates each supported stack separately. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Profile metadata alone does not establish working templates; generation validates each supported stack separately.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across profiles, stack adapter contracts, cli discovery; versioned profile metadata. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
