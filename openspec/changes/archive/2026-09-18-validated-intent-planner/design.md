## Context

Natural language to validated reviewable assembly plans is planned from [requirement.md](../../../requirement.md) §15, §16, §17, §18, §45, §46. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [semantic-component-registry](../semantic-component-registry/proposal.md), [semantic-ui-patterns](../semantic-ui-patterns/proposal.md), [quality-policy-integration](../quality-policy-integration/proposal.md)

## Goals / Non-Goals

**Goals:** Validated intent boundary; Reviewable deterministic plan. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** Complex AI planning in v0.1, automatic publication from natural language, or full-project code generation when mature assets exist.

## Decisions

AI output is untrusted structured Intent: action, project/profile, required and forbidden capabilities and constraints. Validate before planning. Resolver pins compatible assets, prefers Certified and topologically orders steps; attach expected mutations and validation. Only unresolved differences become explicit glue/business-code or semantic-spec work, never raw AI shell commands. Revalidate revision and catalog hashes before execution.

### Contracts and ownership

Affected surfaces: Intent schema, provider adapter, capability graph, deterministic resolver and plan executor. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Validated intent boundary:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Reviewable deterministic plan:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

Provider variability and stale plans can undermine determinism; bind execution to normalized Intent and resolved versions rather than regenerating plans during apply. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- Provider variability and stale plans can undermine determinism; bind execution to normalized Intent and resolved versions rather than regenerating plans during apply.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across intent schema, provider adapter, capability graph, deterministic resolver and plan executor. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
