# Proposal: Make the deploy plane executable through the Jenkins adapter

## Why

Forge's deployment contract invokes an external executor through
`FORGE_DEPLOYER_BIN` (default `forge-deployer`) — a binary that exists
nowhere, so `forge deploy apply|observe` can only ever report
`deploy-target-unavailable`. The workspace's real deployment executor is
jenkins-local: `project.sh <project>` with per-project Jenkins Pipeline jobs
offering deploy/restart/stop/status/logs, and a `deploy-all.sh --dry-run`
preview. Publishing a stable executor contract plus a reference adapter
makes Forge's deployment plane executable on the actual fleet, and gives
jenkins-local's operations Forge's idempotency journal, confirm guards and
timestamped health observations.

## What Changes

- Freeze the executor boundary Forge already calls
  (`apply --target <t> --kind <k> --project <id> --revision <rev>
  --dry-run`, plus `observe`) into a versioned `forge-deploy-executor/0.1.0`
  JSON-envelope contract, documented in
  `docs/adapter-contracts/deploy-executor.md`.
- Ship a reference adapter (`adapters/jenkins/forge-deployer-jenkins` —
  a thin wrapper over jenkins-local's documented `project.sh` verbs, never
  reimplemented logic) in the Forge tree as a tested executable fixture.
- Add provider-matrix wiring so the `deploy` row can reach `supported`
  against the adapter with `--dry-run` only, and document the
  jenkins-local-side promotion to production use as a follow-up owned by
  that repo.
- Health observation maps `project.sh status` output to the existing
  `HealthObservation` vocabulary; unknown output → `unknown`, never healthy.

## BFS Impact Map

- **Capabilities:** `adapter-deployment` (executor contract),
  `provider-integration-evidence` (deploy row semantics).
- **Users and flows:** operators deploying registered projects through the
  Jenkins pipeline; dry-run rehearsal; post-deploy observe.
- **Contracts/data/persistence:** no Forge schema change; adapter state
  stays in Jenkins/Mac host; Forge keeps only its DeployState records.
- **Integrations/configuration:** jenkins-local scripts invoked by path via
  `FORGE_DEPLOYER_BIN` or adapter install; Jenkins secrets stay in Jenkins
  credential stores; Forge receives neither host paths beyond config nor
  secrets.
- **Callers:** CLI `forge deploy …`, MCP (deploy intentionally stays out of
  the mature tool registry until the runtime claim is evidenced), API
  mutation route (confirm-gated, unchanged).
- **Failure/boundary behavior:** adapter exit non-zero with parseable
  envelope → failed stage with evidence; unparseable/timeout →
  `deploy-target-unavailable` with prior state preserved; dry-run never
  triggers a real deploy.
- **Tests:** adapter contract fixtures for apply/observe/dry-run/failed,
  path/argument confinement, redaction of host strings.
- **Dependencies:** none of the other proposed changes; jenkins-local repo
  work is a downstream adoption, not a prerequisite for the Forge-side
  contract + fixture.
- **Compatibility/security/privacy:** existing deploy configs keep working;
  `--confirm` semantics unchanged; no shell interpolation in Forge's
  invocation (already guaranteed).

## Capabilities

- `adapter-deployment`: a published executor contract and a reference
  Jenkins adapter give Forge's deployment plane a real, testable backend.

## Non-goals

- Implementing Jenkins job management, compose or host sync in Forge.
- Replacing jenkins-local or the Jenkins pipeline UI.
- Promoting `deploy` into the MCP mature tool registry in this change.
- Claiming production deployment evidence from a fixture round trip.

Source: requirement.md §30, §32, §43.
