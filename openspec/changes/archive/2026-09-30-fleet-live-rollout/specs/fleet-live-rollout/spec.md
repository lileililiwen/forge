# Capability: fleet-live-rollout

## ADDED Requirements

### Requirement: Concurrent fleet publish

Forge SHALL support `forge publish fleet --jobs N` (default 4,
`--jobs 1` preserves sequential behavior) running per-project
publishes concurrently while writing all journal rows serially on
the main thread with unchanged row shapes and queue id semantics.

#### Scenario: Parallel fleet matches sequential results

- **WHEN** `forge publish fleet --jobs 4 --dry-run --format json`
  runs against the current roster
- **THEN** per-project reports equal the `--jobs 1` reports
  (same stages, same classifications, same counts)

#### Scenario: Fail-fast stops scheduling

- **WHEN** `--fail-fast` is combined with `--jobs 4` and a project fails
- **THEN** no new projects start after the failure is observed and
  the command exits non-zero with completed projects reported

### Requirement: Fit-for-purpose sync timeout

Sync-stage transfers SHALL have a 600s ceiling; fast probes keep
60s; deploy builds keep 1800s.

#### Scenario: Large tree syncs under contention

- **WHEN** an 8-way parallel sync of multi-GB trees runs
- **THEN** no transfer fails closed at 60s; genuine stalls still
  fail at the documented ceiling with the stage named

### Requirement: Working fleet-registry flag

An explicit `--fleet-registry <path>` SHALL load through the
legacy `projects.json` compatibility adapter, never the inventory
branch.

#### Scenario: Filtered registry file

- **WHEN** `--fleet-registry` points at a valid `projects.json`
- **THEN** the fleet classifies its entries (no contract error)

### Requirement: 20/20 live rollout

The Mac fleet SHALL reach 20/20 `healthy=true` via the
`remote-compose` lane with all journal rows `done`, every
`forge-*` compose project brought up by the deploy stage, and
every remediation recorded (secret keys provisioned, legacy
containers/volumes removed, sibling Dockerfile and healthcheck
fixes linked). App-internal runtime failures that survive a
successful deploy stage are out of scope (proposal non-goals).

#### Scenario: Rollout evidence

- **WHEN** the rollout fleet finishes
- **THEN** `deploy status --queue <id>` shows all `done` and the
  change records queue id, per-project verdicts, and the ops log
