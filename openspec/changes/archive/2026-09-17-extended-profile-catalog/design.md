## Context

React profile and extensible specialist profiles is planned from [requirement.md](../../../requirement.md) §9, §13. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [feature-lifecycle](../feature-lifecycle/proposal.md)

## Goals / Non-Goals

**Goals:** React web generation support; Explicit catalog support status. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Implementing all candidate profiles immediately or shipping UI pattern semantics in profile templates.

## Decisions

Add react-web after the five-profile MVP. Reserve aspnet-saas, rust-cli, rust-worker, nextjs-content, python-data, python-ai and flutter-client as explicit future candidates; unsupported candidates remain discoverable as planned, not executable. Reuse versioned profile contracts and publish real native validation evidence for each promoted profile.

### Contracts and ownership

Affected surfaces: Profile descriptors, stack adapters, generation fixtures. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — React web generation support:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Explicit catalog support status:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Profile proliferation can exceed tested support; gate availability on assets and actual verification. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Profile proliferation can exceed tested support; gate availability on assets and actual verification.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across profile descriptors, stack adapters, generation fixtures. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
