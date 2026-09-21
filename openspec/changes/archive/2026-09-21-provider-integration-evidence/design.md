## Context

Forge has adapter boundaries and strong negative-path tests, but those tests
cannot prove the configured external binaries, OIDC issuer, analytics sources
or deployment targets work in a real environment. The roadmap explicitly
requires controlled evidence and partial outcomes.

## Goals / Non-Goals

**Goals:** opt-in real round trips, project-scoped identity and mapping,
credential-safe logs, evidence provenance, and repeatable teardown.

**Non-Goals:** storing provider secrets, replacing external systems, or
claiming all providers are supported because one provider passed.

## Decisions

Integration jobs run against declared sandbox targets and receive secrets only
through the runner's secret mechanism. Forge receives provider references and
redacted receipts, persists timestamps/source/revision and maps failures to the
existing unavailable/unknown/ambiguous/partial states. Identity tests cover
issuer, audience, nonce, expiry, scope, project mismatch and revocation.
Analytics and deployment tests cover provider project mapping and malformed,
timeout and stale observations. A provider matrix records supported,
unavailable and not-run separately; not-run never becomes verified.

## Requirement and scenario coverage

- **R1 — Controlled provider round trips:** successful observations with
  provenance and safe teardown.
- **R2 — Negative and partial integration outcomes:** auth, mapping, timeout,
  malformed output and partial publication cases.

## Failure and compatibility

Local operation without configured providers remains valid and reports
unavailable/disabled as designed. Existing state files remain readable. A
failed provider cannot downgrade another provider's evidence or mark a project
healthy.

## Migration Plan

No mandatory migration. Add opt-in runner configuration and evidence schemas
only after compatibility review; do not add credentials to manifests or the
repository.

## Verification Strategy

Run provider sandboxes where available, all negative authorization/mapping
cases, redaction scans, cross-surface CLI/MCP/API/portal comparisons, and the
full local quality suite. Record exact providers not run.

## Open Questions

Provider sandbox identities, retention duration and approval owners must be
selected by the deployment environment before integration jobs are enabled.
