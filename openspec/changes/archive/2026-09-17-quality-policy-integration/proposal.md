# Proposal: DriftWatch adapter and policy evidence

## Why

[The product brief](../../../requirement.md) §24, §32, §41 requires driftwatch adapter and policy evidence. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.3.

## What Changes

- **Delegated policy execution:** Forge will invoke configured DriftWatch checks and map findings to project observations while retaining category, policy ID, severity, tool version and evidence.
- **Quality result isolation:** Forge will scope policy execution to the selected project and redact credentials from captured evidence.

## BFS Impact Map

- **Capabilities and flows:** DriftWatch adapter and policy evidence; acceptance outcomes are specified in [the capability delta](specs/quality-policy-integration/spec.md).
- **Modules, contracts and persistence:** Integrations, Policies, Doctor, quality observations and process invocation.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [doctor-maturity-assessment](../doctor-maturity-assessment/proposal.md), [feature-lifecycle](../feature-lifecycle/proposal.md)
- **Failure and boundary behavior:** External output and command versions may drift; contract fixtures supplement but do not replace a real integration run.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Reimplementing DriftWatch detectors or reporting absent tooling as a pass.

## Capabilities

### New Capabilities

- `quality-policy-integration`: DriftWatch adapter and policy evidence.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Reimplementing DriftWatch detectors or reporting absent tooling as a pass. This package is planning-only and does not authorize implementation or external operations.
