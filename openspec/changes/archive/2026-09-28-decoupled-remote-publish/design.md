# Design: decoupled-remote-publish

## Context

`forge publish fleet` (and bare `forge publish` from `sibling-cwd-publish`) routes through `src/publish/mod.rs` `PublishAdapter` → `src/publish/jenkins.rs` `JenkinsAdapter` → `src/main.rs:cmd_publish_fleet` → `run_publish`. The `JenkinsAdapter` renders four stages as `ssh mac bash scripts/*.sh` (`project-ports.sh` → `compose-ports.py`/`port_allocator.py`, `shared-postgres.sh`, `project-action.sh` → `project-up.sh`/`project-db-overlay.py`/`docker compose`/`generate-caddyfile.py`). Behaviour (20-port blocks in 10000-19999, `runtime/port-registry.json`, `ports.compose.yml` `!override`, shared-DB overlay, `platform/Caddyfile` wildcard) is owned by `/Users/allen/production/mac-production-deployment` and `/Users/allen/jenkins/scripts` on the target host. Live `alethefy` deploy just emitted invalid `services must be a mapping` from the empty-DB overlay path, and migrating to a cloud host would require reinstalling that script tree. The requirement is ambient in `publish-commands` §30/§32: Forge is the control plane, the target is a directory tree (`projects/<id>/`, `runtime/<id>/`, `platform/`) + Docker, no deployment scripts. This change must not alter roster semantics (`workspace-governance/projects.json` → `filter_eligible` → `compose_ready` 20/68) or the `forge-publish-provider/0.1.0` lane.

## Goals / Non-Goals

**Goals:**
- `forge publish fleet` and bare `forge publish` succeed with zero deployment script checkout on the target. The target holds only Docker + file tree.
- Port allocation (stable 20-port blocks, `port-registry.json`), shared-DB overlay and Caddy host rendering become Forge-owned Rust, validated against current Python/sh fixtures as oracles. Empty-DB overlay emits `services: {}` and validates.
- Secrets stay target-local (`secrets/<project>/.env`, `.shared-db.env`) and are referenced via `--env-file`, never copied on Linux (preserves `publish-commands` Mac-local secret boundary).
- Cloud migration is env-only (`FORGE_PUBLISH_SSH_TARGET`, `REMOTE_ROOT`, `RUNTIME_ROOT`, `PLATFORM_ROOT`, `SECRETS_ROOT`, `SHARED_INFRA_ROOT`, `DOMAIN`, `NAV_HOST`) with Mac-preserving defaults; no `install-mac.sh`.

**Non-Goals:**
- Removing the Mac script tree before the decoupled path is green; rewriting sibling `docker-compose.yml`; inventing GitHub Actions or image-registry publication; changing GitHub push idempotency or `sibling-cwd-publish` cwd-discovery.

## Decisions

### D1 — Supersede `JenkinsAdapter`'s shell plans with a `RemoteComposeAdapter`, keep the `PublishAdapter` trait

**Chosen:** Add `src/publish/remote_compose.rs` (or rename `jenkins.rs` → `remote.rs`) implementing `PublishAdapter` with same ids/stages (`Sync`/`Prepare`/`Db`/`Deploy`/`All`) but plans that are pure `ssh`/`rsync`/`scp`/`docker compose`/`cat` argument arrays. Keep `JenkinsAdapter` compiled behind an explicit adapter-id/env switch for one cycle so rollback is a flag change.

| Stage | Today | Decoupled |
|---|---|---|
| `Sync` | `ssh mkdir` + `rsync -az -e ssh` | same |
| `Prepare` | `ssh bash project-ports.sh` | Forge reads `runtime/port-registry.json` over `ssh cat`, allocates in Rust, renders `ports.compose.yml` locally, ships via `scp`/`ssh install -m 644` |
| `Db` | `ssh bash shared-postgres.sh start/provision` | `ssh /usr/local/bin/docker compose -f shared-infra/compose.yml up -d production-postgres` etc.; per-project overlay in Rust, consumed via compose `-f runtime/<id>/shared-db.compose.yml` |
| `Deploy` | `ssh bash project-action.sh deploy` | `ssh docker compose -p forge-<id> -f projects/<id>/docker-compose.yml -f runtime/<id>/ports.compose.yml [-f runtime/<id>/shared-db.compose.yml] [--env-file runtime/<id>/production.env] up -d --build` (prepending Docker Desktop PATH, `BUILDKIT_PROGRESS=plain`), then Caddy render as above |

**Alternatives:** patch `project-db-overlay.py` in place (leaves coupling); replace rsync with artifact `git push` (deferred); maintain Forge-local `port-registry.json` (diverges from target operator view) — rejected.

### D2 — Rust port allocator owned by Forge, target-hosted registry stays authoritative

**Chosen:** Port `port_allocator.py` `PortAllocator` (SHA256 → block index, `block_size=20`, scan `candidate_base` → free block, legacy-port reuse) into `src/publish/port_allocator.rs`, exercising existing Python test vectors as unit-test oracles. `compose-ports.py` `extract_published_ports` (via `docker compose config --format json`) remains a one-shot `ssh docker compose config` call on the target for the published-port set, but allocation/render moves to Forge. `port-registry.json` stays at `RUNTIME_ROOT/port-registry.json` so concurrent `project-ports.sh` readers during transition don't diverge; registry writes are `scp to *.tmp` + `ssh mv`.

### D3 — Shared-DB overlay and empty-services fix in Forge

**Chosen:** Port `project-db-overlay.py` `database_services` (image `postgres`/`pgvector`), `application_services` (`DATABASE_URL`/`PG*`), `render_overlay` into Rust; `docker compose config --no-interpolate --format json` is still collected on the target, rendered locally, and emitted as `services: {}` when `application_services` is empty (the live `alethefy` bug). Scale args `--scale <db>=0` preserved for DB-offloaded projects.

### D4 — Caddy rendering in Forge, `scp` to target

**Chosen:** Port `generate-caddyfile.py` (registry → `platform/Caddyfile` + `platform/site/index.html` → `docker compose -f platform/compose.yml up -d --force-recreate`) into Rust/askama template, driven by the same `port-registry.json` `ssh cat`. All captured strings through `policy::redact_credentials`.

### D5 — Cloud host is config, not code

**Chosen:** Extend `JenkinsConfig` → `RemoteConfig` (`ssh_target`, `remote_root` `/Users/allen/jenkins/projects`, `runtime_root` `/Users/allen/jenkins/runtime`, `platform_root` `/Users/allen/production/platform`, `secrets_root` `/Users/allen/production/secrets`, `shared_infra_root` `/Users/allen/production/shared-infra`, `domain` `tooosall.uk`, `nav_host` `apps`) with envs `FORGE_PUBLISH_REMOTE_ROOT`/`RUNTIME_ROOT`/… Defaults preserve Mac layout; `_open_spec` template updated accordingly.

## Risks / Trade-offs

- **`port-registry.json` races across fleet workers** → Mitigation: fleet stays serial (`queue_id` ordering); writes atomic (`*.tmp` + `mv`) over `scp`; no parallel deploy.
- **Docker PATH drift over SSH (`docker: command not found`)** → Mitigation: always invoke `/usr/local/bin/docker` and prepend `/Applications/Docker.app/Contents/Resources/bin`, matching `project-up.sh`.
- **Secret leakage via evidence/logs** → Mitigation: all captured stdout/stderr/Caddy evidence through `policy::redact_credentials` before journal or state file; `--env-file` refs never inline.
- **Behaviour drift from Python port-allocator** → Mitigation: Python fixtures become Rust unit-test oracles (block determinism, legacy-port reuse, unavailable-port skipping).
- **Broken transition wipes target evidence** → Mitigation: keep `JenkinsAdapter` behind adapter-id flag for one cycle; delete nothing until fleet 20/20 green; prior `DeployState` preservation rules unchanged.

## Migration Plan

1. Ship `RemoteComposeAdapter` + allocator/overlay/Caddy modules alongside `JenkinsAdapter`; gate by adapter id env so `forge publish fleet --dry-run` can be compared before real deploy.
2. `forge publish fleet --dry-run` must render real `ssh`/`rsync`/`docker compose` lines (no `project-action.sh`) before any `--dry-run=false`.
3. Live canary: single `alethefy` deploy on mounted Mac; then full 20-project fleet; verify `runtime/port-registry.json` (20 entries), `platform/Caddyfile` renders `<project>.tooosall.uk`, `docker ps` shows `forge-<id>` containers, one `<project>.tooosall.uk` subdomain curl 2xx.
4. After green, stop writing Mac script tree; leave it read-only one release, then archive (no publish-logic deletion in this change).

## Open Questions

- Keep `JenkinsAdapter` behind a feature flag beyond one cycle, or remove after fleet green? Recommend keep one cycle.
- Shared-DB overlay sentinel — keep `.shared-db.env` existence (current) vs. `projects.json` field? Keep `.shared-db.env` for this change.
