# Proposal: Real provider integration evidence

## Why

The implemented adapters correctly model unavailable and simulated outcomes,
but the roadmap explicitly requires real DriftWatch, OIDC, analytics and
deployment/provider evidence before claiming integrated support. Current
handoff notes state that these round trips remain downstream steps.

## What Changes

- Define controlled integration fixtures and provider contracts for DriftWatch,
  OIDC identity, analytics/content adapters and deployment/release boundaries.
- Record provider version, configuration source, project mapping, timestamps,
  receipts and partial/unavailable outcomes without exposing credentials.
- Add opt-in integration jobs and evidence review rules; unit fakes remain
  supplemental rather than sufficient.

## BFS Impact Map

- **Capabilities and flows:** doctor/policy, identity session lifecycle,
  analytics observations, deployment health and release checks.
- **Modules, contracts and persistence:** `src/policy`, `src/identity`,
  `src/analytics`, `src/deploy`, `src/release`, registry observations and
  state files.
- **Callers:** CLI first; API/MCP/portal must preserve the same outcomes.
- **Dependencies:** `profile-and-release-readiness`, external provider test
  accounts or local sandboxes, and existing adapter binaries.
- **Failure and boundary behavior:** missing provider, revoked/expired
  credentials, project mismatch, timeout, malformed output and partial stage.
- **Tests:** controlled sandbox round trips, negative authorization tests,
  redaction checks and cross-surface equivalence.
- **Compatibility/security/privacy:** opt-in secrets supplied by the runner,
  never committed or logged; no provider becomes mandatory for local CLI use.
- **Unaffected:** provider ownership, generic CI/CD replacement and portal UI
  framework selection.

## Capabilities

### New Capabilities

- `provider-integration-evidence`: controlled runtime evidence for existing
  integration boundaries.

### Modified Capabilities

- `quality-policy-integration`, `central-admin-identity`,
  `external-planes-analytics`, `adapter-deployment` and
  `release-publishing`: distinguish contract fixtures from verified provider
  round trips.

## Non-goals

This package does not implement a new provider, replace DriftWatch/OpenCode,
create a hosted identity service, or make external access mandatory.
