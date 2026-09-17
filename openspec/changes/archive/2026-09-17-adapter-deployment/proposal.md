# Proposal: Simple deployment targets and observed runtime state

## Why

[The product brief](../../../requirement.md) §30, §34, §43 requires simple deployment targets and observed runtime state. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.5.

## What Changes

- **Explicit adapter deployment:** Forge will implement deploy with project and target selection through versioned Docker Compose and local/SSH adapters using validated artifact and configuration inputs.
- **Observed health and recovery:** Forge will distinguish pending, running, failed and unknown deployment states using timestamped health evidence and expose recovery information for failed rollouts.

## BFS Impact Map

- **Capabilities and flows:** Simple deployment targets and observed runtime state; acceptance outcomes are specified in [the capability delta](specs/adapter-deployment/spec.md).
- **Modules, contracts and persistence:** Deployment adapters, artifact selection, registry observations, target and secret references.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [release-publishing](../release-publishing/proposal.md)
- **Failure and boundary behavior:** Remote hosts are independent failure domains; disconnected means unknown, not offline proof. Recovery may require operator action.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Kubernetes control plane, arbitrary shell from natural language, or distributed scheduling.

## Capabilities

### New Capabilities

- `adapter-deployment`: Simple deployment targets and observed runtime state.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Kubernetes control plane, arbitrary shell from natural language, or distributed scheduling. This package is planning-only and does not authorize implementation or external operations.
