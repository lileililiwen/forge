# Proposal: OIDC admin federation with per-project sessions

## Why

[The product brief](../../../requirement.md) §25, §31 requires oidc admin federation with per-project sessions. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: after the v0.5 lifecycle foundation.

## What Changes

- **Federated admin sign-in:** Forge will support compatible auth/admin features using standard OIDC so an existing provider login can authenticate separate project admin applications.
- **Session and project isolation:** Forge will retain per-project sessions and avoid requiring shared cookies across unrelated applications.

## BFS Impact Map

- **Capabilities and flows:** OIDC admin federation with per-project sessions; acceptance outcomes are specified in [the capability delta](specs/central-admin-identity/spec.md).
- **Modules, contracts and persistence:** Auth/admin feature mappings, OIDC provider configuration and session boundaries.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** [feature-lifecycle](../feature-lifecycle/proposal.md), [adapter-deployment](../adapter-deployment/proposal.md)
- **Failure and boundary behavior:** Provider login does not imply admin authorization; scope claims and application permissions independently.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** SSO in v0.1, cross-application shared cookies, or a home-grown identity protocol.

## Capabilities

### New Capabilities

- `central-admin-identity`: OIDC admin federation with per-project sessions.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

SSO in v0.1, cross-application shared cookies, or a home-grown identity protocol. This package is planning-only and does not authorize implementation or external operations.
