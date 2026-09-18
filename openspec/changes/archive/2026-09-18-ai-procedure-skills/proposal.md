# Proposal: Portable AI procedures over stable operations

## Why

[The product brief](../../../requirement.md) §19, §32 requires portable ai procedures over stable operations. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: after the v0.5 lifecycle foundation.

## What Changes

- **Discoverable operation procedures:** Forge will expose versioned SOPs for the named lifecycle workflows using stable CLI/MCP operations and explicit prerequisites and verification.
- **Procedures do not bypass Core:** Forge will keep implementation and mutation validation in Core and preserve operation-specific execution authority when procedures run.

## BFS Impact Map

- **Capabilities and flows:** Portable AI procedures over stable operations; acceptance outcomes are specified in [the capability delta](specs/ai-procedure-skills/spec.md).
- **Modules, contracts and persistence:** Versioned procedure resources, CLI/MCP references and procedure validation.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [mature-mcp-surface](../mature-mcp-surface/proposal.md), [validated-intent-planner](../validated-intent-planner/proposal.md), [adapter-deployment](../adapter-deployment/proposal.md)
- **Failure and boundary behavior:** Agent prompt compliance is not an enforcement boundary; Core still validates every operation.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Another agent runtime, duplicated generators inside prompts, or binding procedures to one model or IDE.

## Capabilities

### New Capabilities

- `ai-procedure-skills`: Portable AI procedures over stable operations.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Another agent runtime, duplicated generators inside prompts, or binding procedures to one model or IDE. This package is planning-only and does not authorize implementation or external operations.
