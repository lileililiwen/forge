# Proposal: Optional portal for the shared control plane

## Why

[The product brief](../../../requirement.md) §36 requires optional portal for the shared control plane. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: after the v0.5 lifecycle foundation.

## What Changes

- **Shared project and lifecycle views:** Forge will provide the planned portal sections using shared API resources, including project identity, maturity, quality, agent, deployment and documentation status.
- **Accessible controlled operations:** Forge will provide keyboard-accessible responsive operation flows with loading, empty, error, success and partial-result states, preserving Core review and authorization requirements.

## BFS Impact Map

- **Capabilities and flows:** Optional portal for the shared control plane; acceptance outcomes are specified in [the capability delta](specs/control-plane-portal/spec.md).
- **Modules, contracts and persistence:** Portal navigation, API client, accessible operation views and project status.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [core-http-api](../core-http-api/proposal.md), [external-planes-analytics](../external-planes-analytics/proposal.md), [semantic-ui-patterns](../semantic-ui-patterns/proposal.md)
- **Failure and boundary behavior:** A dashboard can imply stronger health than evidence supports; display unknown/stale and partial results prominently.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** A portal in the early MVP, another backend policy engine, or making graphical access mandatory.

## Capabilities

### New Capabilities

- `control-plane-portal`: Optional portal for the shared control plane.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

A portal in the early MVP, another backend policy engine, or making graphical access mandatory. This package is planning-only and does not authorize implementation or external operations.
