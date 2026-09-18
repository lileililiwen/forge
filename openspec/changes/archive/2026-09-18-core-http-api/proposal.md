# Proposal: Optional HTTP transport over Core

## Why

[The product brief](../../../requirement.md) §35 requires optional http transport over core. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: after the v0.5 lifecycle foundation.

## What Changes

- **Equivalent HTTP domain behavior:** Forge will expose stable HTTP project and lifecycle operations using the same validation and results as CLI and MCP.
- **Authorized asynchronous operations:** Forge will enforce project/action authorization and expose operation identity, progress and final outcomes for long-running requests.

## BFS Impact Map

- **Capabilities and flows:** Optional HTTP transport over Core; acceptance outcomes are specified in [the capability delta](specs/core-http-api/spec.md).
- **Modules, contracts and persistence:** API routing, Core contracts, operation resources and access control.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [mature-mcp-surface](../mature-mcp-surface/proposal.md), [adapter-deployment](../adapter-deployment/proposal.md), [external-planes-analytics](../external-planes-analytics/proposal.md)
- **Failure and boundary behavior:** Binding HTTP broadens access beyond a local process; default network exposure and authentication mode must be resolved before enabling a listener.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Duplicating Core logic, requiring HTTP in v0.1, or assuming an unauthenticated public listener is acceptable.

## Capabilities

### New Capabilities

- `core-http-api`: Optional HTTP transport over Core.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Duplicating Core logic, requiring HTTP in v0.1, or assuming an unauthenticated public listener is acceptable. This package is planning-only and does not authorize implementation or external operations.
