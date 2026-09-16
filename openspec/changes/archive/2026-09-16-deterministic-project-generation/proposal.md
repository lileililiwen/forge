# Proposal: Deterministic new-project assembly

## Why

[The product brief](../../../requirement.md) §3, §14, §15, §34, §39 requires deterministic new-project assembly. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.1.

## What Changes

- **Equivalent deterministic creation modes:** Forge will implement forge new with explicit profile selection and interactive input, producing equivalent ordinary source from identical inputs and pinned assets.
- **Portable generated projects:** Forge will generate the five MVP profile projects with manifests and documented native build/test commands, without a Forge runtime dependency.

## BFS Impact Map

- **Capabilities and flows:** Deterministic new-project assembly; acceptance outcomes are specified in [the capability delta](specs/deterministic-project-generation/spec.md).
- **Modules, contracts and persistence:** Generator, CLI interactive and explicit inputs, templates, adapter build/test callers.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [profile-registry](../profile-registry/proposal.md), [project-import](../project-import/proposal.md)
- **Failure and boundary behavior:** Toolchains vary by host; distinguish rendering verification from native build/test evidence for every profile.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** AI generation, installing v0.2 features in v0.1, publishing, or overwriting an existing project.

## Capabilities

### New Capabilities

- `deterministic-project-generation`: Deterministic new-project assembly.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

AI generation, installing v0.2 features in v0.1, publishing, or overwriting an existing project. This package is planning-only and does not authorize implementation or external operations.
