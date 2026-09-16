## Context

Deterministic new-project assembly is planned from [requirement.md](../../../requirement.md) §3, §14, §15, §34, §39. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [profile-registry](../profile-registry/proposal.md), [project-import](../project-import/proposal.md)

## Goals / Non-Goals

**Goals:** Equivalent deterministic creation modes; Portable generated projects. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** AI generation, installing v0.2 features in v0.1, publishing, or overwriting an existing project.

## Decisions

Normalize explicit flags and interactive answers into one creation request. Render pinned profile assets into a staging directory, validate paths, then promote into an empty destination and register. Record the resolved asset versions; generated projects own ordinary source and native tool commands. Feature flags become operational only after feature-lifecycle.

### Contracts and ownership

Affected surfaces: Generator, CLI interactive and explicit inputs, templates, adapter build/test callers. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Equivalent deterministic creation modes:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Portable generated projects:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Toolchains vary by host; distinguish rendering verification from native build/test evidence for every profile. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Toolchains vary by host; distinguish rendering verification from native build/test evidence for every profile.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across generator, cli interactive and explicit inputs, templates, adapter build/test callers. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
