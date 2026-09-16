# Proposal: Versioned profiles for the five MVP stacks

## Why

[The product brief](../../../requirement.md) §3, §9, §37, §39 requires versioned profiles for the five mvp stacks. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.1.

## What Changes

- **MVP profile discovery:** Forge will list and inspect versioned descriptors for all five MVP profiles with supported capabilities, packages, layout, conventions, build/test commands, deployment defaults and quality policies.
- **Stack compatibility contract:** Forge will resolve a profile only when its declared language, toolchain and capability constraints are compatible and will explain unsupported combinations.

## BFS Impact Map

- **Capabilities and flows:** Versioned profiles for the five MVP stacks; acceptance outcomes are specified in [the capability delta](specs/profile-registry/spec.md).
- **Modules, contracts and persistence:** Profiles, stack adapter contracts, CLI discovery; versioned profile metadata.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [core-manifest-registry](../core-manifest-registry/proposal.md)
- **Failure and boundary behavior:** Profile metadata alone does not establish working templates; generation validates each supported stack separately.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** react-web and specialized profiles are later; no feature installer or universal cross-stack implementation.

## Capabilities

### New Capabilities

- `profile-registry`: Versioned profiles for the five MVP stacks.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

react-web and specialized profiles are later; no feature installer or universal cross-stack implementation. This package is planning-only and does not authorize implementation or external operations.
