# Proposal: Findings to deterministic fixes or bounded specs

## Why

[The product brief](../../../requirement.md) §22, §26, §32, §34, §41 requires findings to deterministic fixes or bounded specs. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.3.

## What Changes

- **Traceable spec generation:** Forge will implement spec generate with explicit project selection and generate bounded proposals from capability gaps, policy findings and semantic upgrade conflicts.
- **Remediation routing:** Forge will distinguish supported deterministic fixes, semantic specs and manual intervention and require tests and quality evidence before a remediation is complete.

## BFS Impact Map

- **Capabilities and flows:** Findings to deterministic fixes or bounded specs; acceptance outcomes are specified in [the capability delta](specs/specification-remediation/spec.md).
- **Modules, contracts and persistence:** Spec integration, doctor findings, upgrade conflicts and existing Codex proposal workflow.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [project-upgrade-orchestration](../project-upgrade-orchestration/proposal.md), [quality-policy-integration](../quality-policy-integration/proposal.md)
- **Failure and boundary behavior:** Overlapping findings can create duplicate or contradictory work; merge only compatible scopes and preserve source evidence.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Replacing OpenSpec, generating unbounded rewrites, or running agents simply because a spec was generated.

## Capabilities

### New Capabilities

- `specification-remediation`: Findings to deterministic fixes or bounded specs.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Replacing OpenSpec, generating unbounded rewrites, or running agents simply because a spec was generated. This package is planning-only and does not authorize implementation or external operations.
