## Context

Versioned UI patterns across web and Flutter is planned from [requirement.md](../../../requirement.md) §13, §46, §47. There is no application implementation yet. Prerequisites must be implemented and verified, not merely authored: [semantic-component-registry](../semantic-component-registry/proposal.md), [extended-profile-catalog](../extended-profile-catalog/proposal.md)

## Goals / Non-Goals

**Goals:** Semantic UI catalog and states; Deterministic UI installation. Keep outcome ownership in Core and preserve the [architecture constraints](../../../docs/architecture.md).

**Non-Goals:** A Forge portal, forcing a UI library on all projects, or claiming Blazor support without implementation.

## Decisions

Catalog login/register/forgot-password/dashboard/CRUD/filter/form/settings/profile/billing/empty/success/error/modal/confirm/upload/navigation patterns. Contracts describe typography, spacing, responsive layout, loading/error/success states, form behavior, keyboard/focus, accessibility and navigation. Publish React/Next.js and Flutter mappings independently; Blazor remains an optional future mapping.

### Contracts and ownership

Affected surfaces: UI semantic contracts, component catalog, profile adapters and accessibility fixtures. Public requests carry explicit project/asset identity and validated configuration. Core returns typed outcomes; transports render rather than reinterpret them. Configuration and observations are separate, with provenance for any asserted verification. Only the owning module changes its persistent records; callers consume the resulting contract.

### Requirement and scenario coverage

- **R1 — Semantic UI catalog and states:** implement the observable outcomes in DFS task 2.1; cover successful, rejected and boundary cases in the corresponding spec scenarios.
- **R2 — Deterministic UI installation:** implement the observable outcomes in DFS task 2.2; cover successful, rejected and boundary cases in the corresponding spec scenarios.

### Failure and compatibility

A copied screenshot or HTML fragment is not a verified pattern; each adapter needs state and interaction evidence. Validate preconditions before mutations; preserve prior valid state when preflight fails. Report partial execution explicitly if effects already occurred. Do not downgrade an unsupported schema or overwrite unrelated source to recover.

## Risks / Trade-offs

- A copied screenshot or HTML fragment is not a verified pattern; each adapter needs state and interaction evidence.
- A contract fixture can prove normalization and error mapping but cannot alone establish a working external integration or supported stack.
- Adding another transport must reuse the same validation, ownership and operation boundaries.

## Migration Plan

Introduce this capability after its prerequisites. Version new contracts and retain existing valid project metadata. Identify any format or stored-state migration before applying it, include backups or recovery when applicable, and refuse incompatible input. Read-only operations require no source migration. External side effects, when in scope, need operation records and explicit recovery boundaries.

## Verification Strategy

Map every scenario to an observable fixture or integration check. Verify all named requirements individually, then their combined effects across ui semantic contracts, component catalog, profile adapters and accessibility fixtures. Include compatibility and failure recovery, preserve unrelated files, and distinguish native/runtime evidence from simulated adapter evidence. Run applicable local project checks, the name preflight and strict OpenSpec validation before archive.

## Open Questions

Exact dependency/library versions and external provider protocols are not asserted by this proposal. Inspect actual prerequisites during baseline work and document any material decision before implementation. The foundation owns Rust/toolchain/SQLite choices; later transports and adapters must not silently override them.
