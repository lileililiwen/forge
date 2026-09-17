# Proposal: Project and fleet upgrades with conflict handoff

## Why

[The product brief](../../../requirement.md) §26, §34, §40 requires project and fleet upgrades with conflict handoff. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.2.

## What Changes

- **Deterministic upgrade planning:** Forge will produce pinned upgrade plans with compatibility checks, package/configuration/codemod/schema/replacement steps and recovery implications before applying a project or feature upgrade.
- **Isolated fleet outcomes:** Forge will implement upgrade --all with explicit project selection, per-project journals and distinct success, failure, blocked and skipped results.

## BFS Impact Map

- **Capabilities and flows:** Project and fleet upgrades with conflict handoff; acceptance outcomes are specified in [the capability delta](specs/project-upgrade-orchestration/spec.md).
- **Modules, contracts and persistence:** Upgrade, feature migrations, registry selection, migration state and later spec integration.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [feature-lifecycle](../feature-lifecycle/proposal.md)
- **Failure and boundary behavior:** Database migrations may be irreversible; distinguish reversible file recovery from manual data recovery and require declared migration strategy.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Unreviewed fleet mutation, AI editing on conflict, or claiming filesystem rollback reverses database migrations.

## Capabilities

### New Capabilities

- `project-upgrade-orchestration`: Project and fleet upgrades with conflict handoff.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Unreviewed fleet mutation, AI editing on conflict, or claiming filesystem rollback reverses database migrations. This package is planning-only and does not authorize implementation or external operations.
