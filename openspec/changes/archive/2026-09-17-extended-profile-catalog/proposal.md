# Proposal: React profile and extensible specialist profiles

## Why

[The product brief](../../../requirement.md) §9, §13 requires react profile and extensible specialist profiles. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.2.

## What Changes

- **React web generation support:** Forge will add a versioned react-web descriptor and deterministic native-buildable template using the established profile contract.
- **Explicit catalog support status:** Forge will distinguish supported profiles from proposed specialist profiles and require compatibility, template and validation evidence before promotion.

## BFS Impact Map

- **Capabilities and flows:** React profile and extensible specialist profiles; acceptance outcomes are specified in [the capability delta](specs/extended-profile-catalog/spec.md).
- **Modules, contracts and persistence:** Profile descriptors, stack adapters, generation fixtures.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [feature-lifecycle](../feature-lifecycle/proposal.md)
- **Failure and boundary behavior:** Profile proliferation can exceed tested support; gate availability on assets and actual verification.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Implementing all candidate profiles immediately or shipping UI pattern semantics in profile templates.

## Capabilities

### New Capabilities

- `extended-profile-catalog`: React profile and extensible specialist profiles.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Implementing all candidate profiles immediately or shipping UI pattern semantics in profile templates. This package is planning-only and does not authorize implementation or external operations.
