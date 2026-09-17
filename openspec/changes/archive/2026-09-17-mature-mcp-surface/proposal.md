# Proposal: One MCP server over mature Core operations

## Why

[The product brief](../../../requirement.md) §20, §34, §42 requires one mcp server over mature core operations. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.4.

## What Changes

- **Shared validated MCP operations:** Forge will expose stable mature operations through a single MCP server delegating to the same Core contracts used by CLI.
- **Mutating tool boundaries:** Forge will distinguish read-only tools from mutating and external-write tools and bind execution to the selected project and approved operation.

## BFS Impact Map

- **Capabilities and flows:** One MCP server over mature Core operations; acceptance outcomes are specified in [the capability delta](specs/mature-mcp-surface/spec.md).
- **Modules, contracts and persistence:** MCP stdio transport, Core dispatch, tool metadata, operation authorization.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [agent-runtime-workflows](../agent-runtime-workflows/proposal.md)
- **Failure and boundary behavior:** Model-issued requests remain untrusted structured inputs; require scoped execution authority and redact logs.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Full autonomous orchestration, unstable internal endpoints, or requiring HTTP for the first MCP version.

## Capabilities

### New Capabilities

- `mature-mcp-surface`: One MCP server over mature Core operations.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Full autonomous orchestration, unstable internal endpoints, or requiring HTTP for the first MCP version. This package is planning-only and does not authorize implementation or external operations.
