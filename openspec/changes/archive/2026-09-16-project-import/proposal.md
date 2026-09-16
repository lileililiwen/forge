# Proposal: Conservative onboarding of existing repositories

## Why

[The product brief](../../../requirement.md) §8, §34, §39 requires conservative onboarding of existing repositories. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.1.

## What Changes

- **Evidence-backed detection:** Forge will implement forge import with evidence-backed detection of the complete import inventory and a suggested profile and maturity that distinguishes unknown from missing.
- **Minimal and repeatable adoption:** Forge will create a validated manifest and register the project only after its import proposal is accepted; repeated import will preserve identity and unrelated content.

## BFS Impact Map

- **Capabilities and flows:** Conservative onboarding of existing repositories; acceptance outcomes are specified in [the capability delta](specs/project-import/spec.md).
- **Modules, contracts and persistence:** Import detection, filesystem and Git inspection, manifest and registry callers.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [core-manifest-registry](../core-manifest-registry/proposal.md), [profile-registry](../profile-registry/proposal.md)
- **Failure and boundary behavior:** Monorepos and mixed frameworks can produce ambiguous profiles; never resolve by silently taking the first detector.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Reformatting source, dependency installation, changing remotes, or automatic maturity promotion.

## Capabilities

### New Capabilities

- `project-import`: Conservative onboarding of existing repositories.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Reformatting source, dependency installation, changing remotes, or automatic maturity promotion. This package is planning-only and does not authorize implementation or external operations.
