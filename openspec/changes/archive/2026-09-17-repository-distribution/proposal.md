# Proposal: Canonical GitHub repository and one-way Gitee mirrors

## Why

[The product brief](../../../requirement.md) §27, §32, §34, §43 requires canonical github repository and one-way gitee mirrors. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.5.

## What Changes

- **Canonical one-way distribution:** Forge will preserve a configured canonical primary and synchronize selected refs one-way to enabled mirrors, initially GitHub to Gitee.
- **Partial failure and credentials:** Forge will report per-remote outcomes and support retrying failed mirror delivery without misreporting primary state or exposing credentials.

## BFS Impact Map

- **Capabilities and flows:** Canonical GitHub repository and one-way Gitee mirrors; acceptance outcomes are specified in [the capability delta](specs/repository-distribution/spec.md).
- **Modules, contracts and persistence:** Distribution, Git adapters, credential references and remote operation records.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [agent-runtime-workflows](../agent-runtime-workflows/proposal.md)
- **Failure and boundary behavior:** A primary push and mirror push are not atomic; record primary success separately and allow safe mirror retry.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Bidirectional synchronization, implicit force pushes, or storing access tokens in manifests.

## Capabilities

### New Capabilities

- `repository-distribution`: Canonical GitHub repository and one-way Gitee mirrors.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Bidirectional synchronization, implicit force pushes, or storing access tokens in manifests. This package is planning-only and does not authorize implementation or external operations.
