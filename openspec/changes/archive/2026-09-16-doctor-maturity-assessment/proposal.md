# Proposal: Read-only health and maturity assessment

## Why

[The product brief](../../../requirement.md) §23, §25, §34, §39 requires read-only health and maturity assessment. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.1.

## What Changes

- **Doctor finding inventory:** Forge will implement forge doctor checking manifest, missing/outdated features, dependency drift, build/deployment configuration, repository, CI, documentation and maturity requirements, returning evidence and remediation classification.
- **Evidence-based maturity levels:** Forge will assess L0 prototype, L1 structure/configuration/database where applicable/logging/health/build definition, L2 applicable auth/admin/CI/DriftWatch/deployment/audit, L3 identity compatibility/observability/release/registry/distribution/upgrades, and L4 applicable backup/recovery/monitoring/security/privacy/secrets/alerts against current and target levels.

## BFS Impact Map

- **Capabilities and flows:** Read-only health and maturity assessment; acceptance outcomes are specified in [the capability delta](specs/doctor-maturity-assessment/spec.md).
- **Modules, contracts and persistence:** Doctor, profile checks, CLI, registry health observations; maturity policy descriptors.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [deterministic-project-generation](../deterministic-project-generation/proposal.md)
- **Failure and boundary behavior:** Configured maturity is an intent, not proof; stale observations must be shown as stale.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Applying fixes, implementing a competing quality engine, or automatic L0-to-L4 promotion.

## Capabilities

### New Capabilities

- `doctor-maturity-assessment`: Read-only health and maturity assessment.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Applying fixes, implementing a competing quality engine, or automatic L0-to-L4 promotion. This package is planning-only and does not authorize implementation or external operations.
