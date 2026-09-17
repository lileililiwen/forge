# Proposal: Semantic components and evidence-backed quality levels

## Why

[The product brief](../../../requirement.md) §3, §11, §12, §45, §47 requires semantic components and evidence-backed quality levels. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: after the v0.5 lifecycle foundation.

## What Changes

- **Semantic reusable unit contracts:** Forge will register meaningful components with explicit inputs/outputs, tests, versions, compatibility and deterministic installation, linked to features, patterns and capabilities.
- **Quality classification and selection:** Forge will track Experimental, Verified, Certified and Deprecated states with supporting evidence and prefer compatible Certified components without violating constraints.

## BFS Impact Map

- **Capabilities and flows:** Semantic components and evidence-backed quality levels; acceptance outcomes are specified in [the capability delta](specs/semantic-component-registry/spec.md).
- **Modules, contracts and persistence:** Components, capability contracts, catalog provenance and resolver ranking.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [feature-lifecycle](../feature-lifecycle/proposal.md), [project-upgrade-orchestration](../project-upgrade-orchestration/proposal.md)
- **Failure and boundary behavior:** Certification can become stale or conflict with version constraints; compatibility and explicit policy precede quality preference.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Registering programming primitives such as if/loop/string concatenation, or blanket claims of component safety.

## Capabilities

### New Capabilities

- `semantic-component-registry`: Semantic components and evidence-backed quality levels.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Registering programming primitives such as if/loop/string concatenation, or blanket claims of component safety. This package is planning-only and does not authorize implementation or external operations.
