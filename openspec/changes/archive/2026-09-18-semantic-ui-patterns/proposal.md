# Proposal: Versioned UI patterns across web and Flutter

## Why

[The product brief](../../../requirement.md) §13, §46, §47 requires versioned ui patterns across web and flutter. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: after the v0.5 lifecycle foundation.

## What Changes

- **Semantic UI catalog and states:** Forge will describe reusable UI patterns through data, interaction, typography, spacing, responsive, loading, error, success, form, accessibility and navigation contracts.
- **Deterministic UI installation:** Forge will install pinned compatible pattern implementations that remain editable ordinary source and preserve the host project design conventions.

## BFS Impact Map

- **Capabilities and flows:** Versioned UI patterns across web and Flutter; acceptance outcomes are specified in [the capability delta](specs/semantic-ui-patterns/spec.md).
- **Modules, contracts and persistence:** UI semantic contracts, component catalog, profile adapters and accessibility fixtures.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [semantic-component-registry](../semantic-component-registry/proposal.md), [extended-profile-catalog](../extended-profile-catalog/proposal.md)
- **Failure and boundary behavior:** A copied screenshot or HTML fragment is not a verified pattern; each adapter needs state and interaction evidence.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** A Forge portal, forcing a UI library on all projects, or claiming Blazor support without implementation.

## Capabilities

### New Capabilities

- `semantic-ui-patterns`: Versioned UI patterns across web and Flutter.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

A Forge portal, forcing a UI library on all projects, or claiming Blazor support without implementation. This package is planning-only and does not authorize implementation or external operations.
