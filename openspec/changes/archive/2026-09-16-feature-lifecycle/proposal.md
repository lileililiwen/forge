# Proposal: Versioned feature add, remove and upgrade

## Why

[The product brief](../../../requirement.md) §10, §14, §15, §34, §40 requires versioned feature add, remove and upgrade. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.2.

## What Changes

- **Feature resolution and discovery:** Forge will expose versioned feature descriptors with compatibility, dependencies, conflicts, strategies, validation, documentation and tests; only tested profile mappings will be installable.
- **Safe lifecycle operations:** Forge will implement feature add, remove and upgrade and new-project feature selection through reviewable deterministic plans, ownership records and post-change validation.

## BFS Impact Map

- **Capabilities and flows:** Versioned feature add, remove and upgrade; acceptance outcomes are specified in [the capability delta](specs/feature-lifecycle/spec.md).
- **Modules, contracts and persistence:** Features registry, resolver, generator, CLI, manifests and adapter validation.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [doctor-maturity-assessment](../doctor-maturity-assessment/proposal.md)
- **Failure and boundary behavior:** Shared file edits and reverse dependencies make removal hazardous; preserve user edits and stop on ownership conflicts.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Claiming every listed feature works on every stack; bespoke auth generation when a verified supported asset exists.

## Capabilities

### New Capabilities

- `feature-lifecycle`: Versioned feature add, remove and upgrade.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Claiming every listed feature works on every stack; bespoke auth generation when a verified supported asset exists. This package is planning-only and does not authorize implementation or external operations.
