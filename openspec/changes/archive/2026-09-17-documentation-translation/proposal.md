# Proposal: Derivative documentation and source-hash freshness

## Why

[The product brief](../../../requirement.md) §28, §34, §43 requires derivative documentation and source-hash freshness. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.5.

## What Changes

- **Canonical and derivative documentation:** Forge will implement docs translate with explicit locale enablement, derivative paths, source hashes and stale status on source changes.
- **Incremental and reliable translation:** Forge will translate changed segments, preserve unchanged segments and technical literals, and leave existing outputs intact on provider failure.

## BFS Impact Map

- **Capabilities and flows:** Derivative documentation and source-hash freshness; acceptance outcomes are specified in [the capability delta](specs/documentation-translation/spec.md).
- **Modules, contracts and persistence:** Documentation metadata, translation provider adapter, CLI and doctor freshness.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [core-manifest-registry](../core-manifest-registry/proposal.md)
- **Failure and boundary behavior:** AI translation can alter commands or links; preserve fenced code, destinations and explicit non-translatable terms and expose review state.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Translation enabled by default, overwriting canonical docs, or claiming translation quality from provider success alone.

## Capabilities

### New Capabilities

- `documentation-translation`: Derivative documentation and source-hash freshness.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Translation enabled by default, overwriting canonical docs, or claiming translation quality from provider success alone. This package is planning-only and does not authorize implementation or external operations.
