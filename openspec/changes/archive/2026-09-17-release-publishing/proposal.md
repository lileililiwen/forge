# Proposal: Gated resumable releases and publication

## Why

[The product brief](../../../requirement.md) §29, §34, §43 requires gated resumable releases and publication. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.5.

## What Changes

- **Verified release preparation:** Forge will implement release preparation that binds semver, changelog, source revision, configured documentation and doctor/test/DriftWatch evidence before release side effects.
- **Resumable multi-destination publication:** Forge will execute authorized commit, tag, primary push, mirror, package, container and release-note stages with per-stage records and safe retry.

## BFS Impact Map

- **Capabilities and flows:** Gated resumable releases and publication; acceptance outcomes are specified in [the capability delta](specs/release-publishing/spec.md).
- **Modules, contracts and persistence:** Release state machine, version/changelog, Git distribution, package/container providers.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [repository-distribution](../repository-distribution/proposal.md), [documentation-translation](../documentation-translation/proposal.md), [quality-policy-integration](../quality-policy-integration/proposal.md)
- **Failure and boundary behavior:** Published artifacts and pushed tags may be irreversible; resumable stages need idempotent provider operations, not fictional rollback.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Publishing during docs bootstrap, force replacing tags, or treating partial publication as a complete release.

## Capabilities

### New Capabilities

- `release-publishing`: Gated resumable releases and publication.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Publishing during docs bootstrap, force replacing tags, or treating partial publication as a complete release. This package is planning-only and does not authorize implementation or external operations.
