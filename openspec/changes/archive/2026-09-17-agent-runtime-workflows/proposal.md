# Proposal: Existing agent manager and controlled developer operations

## Why

[The product brief](../../../requirement.md) §21, §22, §32, §34, §41 requires existing agent manager and controlled developer operations. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.3.

## What Changes

- **Managed agent sessions:** Forge will expose start, pause, takeover, resume, restart and new-session operations for supported OpenCode/Codex adapters, and route spec run to the selected project and active spec.
- **Verified test and Git operations:** Forge will expose test, commit and push as separate scoped operations, requiring explicit execution intent for remote writes and preserving unrelated work.

## BFS Impact Map

- **Capabilities and flows:** Existing agent manager and controlled developer operations; acceptance outcomes are specified in [the capability delta](specs/agent-runtime-workflows/spec.md).
- **Modules, contracts and persistence:** Agent adapter, existing PTY manager, OpenCode/Codex, test and Git operations.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [specification-remediation](../specification-remediation/proposal.md)
- **Failure and boundary behavior:** Providers differ in pause and takeover support; return explicit unsupported states rather than simulating success.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Building another PTY manager, automatic push after tests, or including unrelated changes in commits.

## Capabilities

### New Capabilities

- `agent-runtime-workflows`: Existing agent manager and controlled developer operations.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Building another PTY manager, automatic push after tests, or including unrelated changes in commits. This package is planning-only and does not authorize implementation or external operations.
