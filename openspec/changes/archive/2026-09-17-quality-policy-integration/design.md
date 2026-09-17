## Context

DriftWatch adapter and policy evidence is planned from [requirement.md](../../../requirement.md) §24, §32, §41. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [doctor-maturity-assessment](../doctor-maturity-assessment/proposal.md), [feature-lifecycle](../feature-lifecycle/proposal.md)

## Goals / Non-Goals

**Goals:** Delegated policy execution; Quality result isolation. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Reimplementing DriftWatch detectors or reporting absent tooling as a pass.

## Decisions

Delegate architecture/security/privacy/dependency/runtime/spec/documentation/deployment/accessibility policies to DriftWatch through a versioned adapter. Normalize external findings while preserving original rule IDs, severity, version and evidence. Use argument arrays, project-scoped working directories and bounded processes; configure the actual discovered interface before implementation.

### Contracts and ownership

Affected surfaces: Integrations, Policies, Doctor, quality observations and process invocation. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Delegated policy execution:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Quality result isolation:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

External output and command versions may drift; contract fixtures supplement but do not replace a real integration run. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- External output and command versions may drift; contract fixtures supplement but do not replace a real integration run.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across integrations, policies, doctor, quality observations and process invocation. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
