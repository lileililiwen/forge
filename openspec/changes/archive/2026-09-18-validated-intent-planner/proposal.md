# Proposal: Natural language to validated reviewable assembly plans

## Why

[The product brief](../../../requirement.md) §15, §16, §17, §18, §45, §46 requires natural language to validated reviewable assembly plans. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: after the v0.5 lifecycle foundation.

## What Changes

- **Validated intent boundary:** Forge will turn natural-language requests into schema-validated Intent containing required/forbidden capabilities and constraints before any project mutation or command execution.
- **Reviewable deterministic plan:** Forge will resolve validated Intent into pinned dependency-ordered assembly and validation steps, explain component choices and expose unresolved custom work.

## BFS Impact Map

- **Capabilities and flows:** Natural language to validated reviewable assembly plans; acceptance outcomes are specified in [the capability delta](specs/validated-intent-planner/spec.md).
- **Modules, contracts and persistence:** Intent schema, provider adapter, capability graph, deterministic resolver and plan executor.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [semantic-component-registry](../semantic-component-registry/proposal.md), [semantic-ui-patterns](../semantic-ui-patterns/proposal.md), [quality-policy-integration](../quality-policy-integration/proposal.md)
- **Failure and boundary behavior:** Provider variability and stale plans can undermine determinism; bind execution to normalized Intent and resolved versions rather than regenerating plans during apply.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Complex AI planning in v0.1, automatic publication from natural language, or full-project code generation when mature assets exist.

## Capabilities

### New Capabilities

- `validated-intent-planner`: Natural language to validated reviewable assembly plans.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Complex AI planning in v0.1, automatic publication from natural language, or full-project code generation when mature assets exist. This package is planning-only and does not authorize implementation or external operations.
