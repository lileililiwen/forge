# Proposal: Existing content and analytics planes with project metrics

## Why

[The product brief](../../../requirement.md) §32, §33 requires existing content and analytics planes with project metrics. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: after the v0.5 lifecycle foundation.

## What Changes

- **External system identity and health:** Forge will integrate existing content and analytics providers through configured adapters retaining external identity, ownership and project mapping.
- **Timestamped project metrics:** Forge will aggregate project, agent, spec, quality, deployment and repository-star metrics with timestamps, source and explicit unavailable/stale states.

## BFS Impact Map

- **Capabilities and flows:** Existing content and analytics planes with project metrics; acceptance outcomes are specified in [the capability delta](specs/external-planes-analytics/spec.md).
- **Modules, contracts and persistence:** Integrations, project ID mapping, analytics snapshots and external content references.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [agent-runtime-workflows](../agent-runtime-workflows/proposal.md), [adapter-deployment](../adapter-deployment/proposal.md), [repository-distribution](../repository-distribution/proposal.md)
- **Failure and boundary behavior:** Metrics may have different windows and unavailable providers; do not sum unknown values into authoritative totals.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Rewriting analytics/content services, inventing production telemetry, or executing remote writes during metric retrieval.

## Capabilities

### New Capabilities

- `external-planes-analytics`: Existing content and analytics planes with project metrics.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Rewriting analytics/content services, inventing production telemetry, or executing remote writes during metric retrieval. This package is planning-only and does not authorize implementation or external operations.
