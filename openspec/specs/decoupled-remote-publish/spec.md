# decoupled-remote-publish Specification

## Purpose
A Forge-owned `remote-compose` publish lane that allocates ports, renders the shared-DB overlay and Caddy hosts, and issues plain `docker compose`/`rsync` over SSH, so the target is a generic Docker host with no deployment scripts on it.
## Requirements
### Requirement: Remote-compose publish without target-hosted deployment scripts
Forge SHALL publish the fleet (and bare `forge publish` via cwd discovery) to a generic Docker host without invoking any file under the target's `scripts/` directory; `Sync` SHALL remain `mkdir` + `rsync`, while `Prepare`/`Db`/`Deploy` SHALL be expressed as plain `ssh`/`rsync`/`scp` + `docker compose` commands issued from Linux.

#### Scenario: No script invoked on target
- **WHEN** `forge publish fleet --dry-run` (or a real fleet/canary) is invoked
- **THEN** no `ssh … bash …/project-ports.sh` / `shared-postgres.sh` / `project-action.sh` command is emitted or executed and the dry-run plan renders `ssh`/`rsync`/`docker compose` lines

#### Scenario: Single-project decoupled deploy
- **WHEN** `forge publish` is run from a sibling checkout (cwd-discovered) and the target is the configured remote-compose host
- **THEN** Forge syncs the project, ships its `ports.compose.yml`, and runs `docker compose -p forge-<id> … up -d --build` on the target without calling `project-action.sh`

### Requirement: Forge-owned port allocation from the target-hosted registry
`Prepare` SHALL be fulfilled by Forge reading `RUNTIME_ROOT/port-registry.json` from the target over SSH, deterministically allocating a stable 20-port block in 10000-19999 for the project (SHA256 identity `local:<project>`), reusing any legacy published port when free, and rendering `ports.compose.yml` with `!override` semantics locally before shipping it to `RUNTIME_ROOT/<project>/ports.compose.yml`.

#### Scenario: Deterministic allocation on existing roster
- **WHEN** `forge publish fleet` allocates ports for two runs over the same `port-registry.json` with no new services
- **THEN** each project receives the same host ports (block-stable) and `ports.compose.yml` is byte-stable

#### Scenario: Registry remains the source of truth
- **WHEN** two fleet invocations run serially against the same target
- **THEN** the second run reads the `port-registry.json` left by the first run and never fabricates a divergent Forge-local registry

### Requirement: Shared-DB overlay rendered in Forge with empty-services validity
For projects that declare shared-DB intent via the target-local `secrets/<project>/.shared-db.env` file, Forge SHALL collect the project's `docker compose config --no-interpolate --format json` from the target, render the shared-DB overlay locally (PG env mapping, `depends_on` pruning with `!reset []` when empty, `production-db-network` injection), and emit valid YAML; when the Compose service set exposes no DB-relevant env, the overlay SHALL be `services: {}` (not `services:\nnetworks:`), which MUST pass YAML validation and `docker compose config`.

#### Scenario: Empty-overlay validity for alethefy-class projects
- **WHEN** a project's Compose file has no `DATABASE_URL`/`PG*`-relevant env and no DB-relevant `depends_on`
- **THEN** the emitted overlay is `services: {}` plus the external `production-db-network` block, validates as YAML, and `docker compose -p forge-alethefy -f docker-compose.yml -f ports.compose.yml -f shared-db.compose.yml config` succeeds

#### Scenario: Overlay faithfulness
- **WHEN** the overlay is rendered for a project that does expose DB-relevant env
- **THEN** it maps `DATABASE_URL` → `${PRODUCTION_DATABASE_URL}`, `PGDATABASE` → `${PRODUCTION_DB_NAME}`, etc., prunes DB `depends_on`, and injects `production-db-network`

### Requirement: Caddy host rendering in Forge
After successful per-project deploys, Forge SHALL render the host's `platform/Caddyfile` (and `platform/site/index.html` when the template exists) from `RUNTIME_ROOT/port-registry.json` and ship it to the target followed by `docker compose -f PLATFORM_ROOT/compose.yml up -d --force-recreate`.

#### Scenario: Caddy renders after fleet
- **WHEN** `forge publish fleet` finishes with 20/20 healthy
- **THEN** `PLATFORM_ROOT/Caddyfile` contains one host rule per `compose_ready` project at `<project>.tooosall.uk` and the `apps` nav host

### Requirement: Secrets remain target-local
Forge SHALL NOT read or copy `SECRETS_ROOT/<project>/.env` or `.shared-db.env` contents over SSH; deploys SHALL reference them via `--env-file RUNTIME_ROOT/<project>/production.env` (or `--env-file SECRETS_ROOT/<project>/.env`), and no secret value SHALL appear in the journal, evidence, or Caddy rendering.

#### Scenario: No secret leakage
- **WHEN** a deploy runs for a project with `SECRETS_ROOT/<project>/.env` present
- **THEN** the journal `detail`, evidence lines, and platform rendering contain `[REDACTED]` or path references, never the secret bytes

### Requirement: Generic-target cloud portability via env
A new cloud host SHALL be targeted by changing only `FORGE_PUBLISH_SSH_TARGET`, `FORGE_PUBLISH_REMOTE_ROOT`, `FORGE_PUBLISH_RUNTIME_ROOT`, `FORGE_PUBLISH_PLATFORM_ROOT`, `FORGE_PUBLISH_SECRETS_ROOT`, `FORGE_PUBLISH_SHARED_INFRA_ROOT`, `FORGE_PUBLISH_DOMAIN`, `FORGE_PUBLISH_NAV_HOST` (all defaulting to the current Mac layout); no `install-mac.sh` or script checkout on the target SHALL be required.

#### Scenario: Env-only migration
- **WHEN** `FORGE_PUBLISH_SSH_TARGET=cloud` and its peer envs point at cloud paths
- **THEN** `forge publish fleet --dry-run` renders `cloud:<remote_root>/<project>/` and cloud runtime/secret paths without any shell script path

### Requirement: Contract preservation for roster, journal, and wildcard routing
`forge publish fleet` SHALL continue to derive the roster from `workspace-governance/projects.json` (or explicit `--inventory` / `--fleet-registry`), filter to `compose_ready` entries only, report every skipped entry explicitly, persist the existing queue/phase `operations` journal without new columns, and route public HTTP hosts through the existing wildcard Cloudflare → Caddy boundary.

#### Scenario: Fleet eligibility unchanged
- **WHEN** `forge publish fleet --dry-run --format json` is run against the current `workspace-governance/projects.json`
- **THEN** it reports `compose_ready=20`, `skipped=48` with each skipped entry's typed classification, and only `compose_ready` entries receive a journal row

### Requirement: Legacy Mac-script adapter retained for one-cycle rollback
`JenkinsAdapter` (`ssh bash scripts/*.sh`) SHALL remain compiled and selectable for one release cycle after this change so a real-deploy failure can be reverted by an adapter-id/env flag without rebuilding Forge.

#### Scenario: Rollback is a flag
- **WHEN** the decoupled adapter is rolled back via adapter id or env
- **THEN** fleet dry-run emits the legacy `project-action.sh` lines again and live deploy follows the previous Mac-script path

