## Context

Read-only health and maturity assessment is planned from [requirement.md](../../../requirement.md) §23, §25, §34, §39. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [deterministic-project-generation](../deterministic-project-generation/proposal.md)

## Goals / Non-Goals

**Goals:** Doctor finding inventory; Evidence-based maturity levels. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Applying fixes, implementing a competing quality engine, or automatic L0-to-L4 promotion.

## Decisions

Use typed findings with rule ID, PASS/WARN/FAIL, evidence, applicability and remediation class (automatic, AI, manual). Unknown or unexecuted checks are represented explicitly and never converted to PASS. Assess current versus requested maturity without enforcing higher targets or requiring database/auth where inapplicable. Doctor is local inspection in v0.1; DriftWatch execution belongs to v0.3.

### Contracts and ownership

Affected surfaces: Doctor, profile checks, CLI, registry health observations; maturity policy descriptors. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Doctor finding inventory:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Evidence-based maturity levels:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Configured maturity is an intent, not proof; stale observations must be shown as stale. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Configured maturity is an intent, not proof; stale observations must be shown as stale.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across doctor, profile checks, cli, registry health observations; maturity policy descriptors. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
