## Context

Semantic components and evidence-backed quality levels is planned from [requirement.md](../../../requirement.md) §3, §11, §12, §45, §47. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [feature-lifecycle](../feature-lifecycle/proposal.md), [project-upgrade-orchestration](../project-upgrade-orchestration/proposal.md)

## Goals / Non-Goals

**Goals:** Semantic reusable unit contracts; Quality classification and selection. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Registering programming primitives such as if/loop/string concatenation, or blanket claims of component safety.

## Decisions

Model Profile > Feature > Pattern > Capability associations without forcing one asset size. Components have semantic input/output contracts, versions, dependencies, compatibility, deterministic installer and validators. Record Experimental/Verified/Certified/Deprecated with usage, coverage, last verification, known issues, supported profiles and security review; quality labels require provenance.

### Contracts and ownership

Affected surfaces: Components, capability contracts, catalog provenance and resolver ranking. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Semantic reusable unit contracts:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Quality classification and selection:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Certification can become stale or conflict with version constraints; compatibility and explicit policy precede quality preference. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Certification can become stale or conflict with version constraints; compatibility and explicit policy precede quality preference.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across components, capability contracts, catalog provenance and resolver ranking. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
