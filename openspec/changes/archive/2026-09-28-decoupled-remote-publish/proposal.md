# Proposal: decoupled-remote-publish

## Why

`forge publish fleet` currently delegates orchestration to Mac-hosted scripts (`project-ports.sh`, `shared-postgres.sh`, `project-action.sh`, `project-db-overlay.py`, `generate-caddyfile.py`). The target holds deployment logic, so migrating to a cloud host requires re-installing the script tree and re-proving behaviour through SSH exits. Forge should own port allocation, DB overlay and Caddy rendering and issue plain `docker compose`/`rsync` commands over SSH so the target is a dumb Docker host swappable by environment alone (`forge publish fleet` from Forge stays the fleet roster, via `workspace-governance/projects.json` → `compose_ready` filter, unchanged by `sibling-cwd-publish`).

## What Changes

- Add a Forge-owned remote-compose provider that implements the existing `PublishAdapter` / `PublishAction` {`Sync`,`Db`,`Prepare`,`Deploy`,`All`} contract without invoking any file under the target's `scripts/` directory. `Sync` stays `mkdir` + `rsync`; `Prepare` reads the target's `runtime/port-registry.json` over SSH, allocates a stable 20-port block (10000-19999) in Rust, renders `ports.compose.yml` locally and ships it; `Db` and `Deploy` invoke `docker compose` directly on the target (shared-infra `production-postgres`, per-project overlays, `--env-file` referencing target-local secrets, `BUILDKIT_PROGRESS=plain`; overlay renders empty `services: {}` correctly when no app DB env is present).
- Port `port_allocator.py` (`PortAllocator` + block hashing) and `project-db-overlay.py` (`database_services`/`application_services`/`render_overlay`) into Rust modules (`src/publish/port_allocator.rs`, `src/publish/db_overlay.rs`) validated against existing Python/Caddy fixtures; vendor or template the Caddy rendering inside Forge so `platform/Caddyfile` is also Forge-owned.
- Keep `JenkinsAdapter` compiled during transition behind an explicit switch (adapter id / env) so rollback is a flag change; delete no Mac script until fleet is green. Secrets remain target-local (`secrets/<project>/.env`, `.shared-db.env`) and are referenced via `--env-file`, never copied — preserving the `publish-commands` Mac-local secret boundary.
- Expose the new `FORGE_PUBLISH_RUNTIME_ROOT` / `PLATFORM_ROOT` / `SECRETS_ROOT` / `SHARED_INFRA_ROOT` envs (with Mac-preserving defaults) so cloud migration is env-only, no `install-mac.sh`.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `decoupled-remote-publish` (Forge Rust) | Forge publishes the full fleet to any generic Docker host without target-hosted deployment scripts, verifiably by subdomain | Forge Rust; target host owns only Docker + file tree | `publish-commands` (Linux-controlled runtime, artifact-only, local-build, secret boundary), existing `PublishAdapter` | `forge-publish-plugin-orchestration`, `forge-independent-project-inventory-fleet`, `sibling-cwd-publish` | Rust unit tests + `forge publish` dry-run shape + live Mac fleet → subdomain |
| Mac runtime host (separate) | None — read-only during this change | jenkins-local / Mac `production/mac-production-deployment` | Not in scope for this proposal | Forge | Not claimed |

This package does not move secrets, rewrite sibling compose files, or claim GitHub push idempotency changes.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge `JenkinsAdapter` | `src/publish/jenkins.rs`, `src/publish/mod.rs` (`PublishAdapter`) | SSH/rsync transport, journal/queue projection, stage classification | Invokes `scripts/*.sh` by name; no port/overlay/Caddy logic in Forge | Forge | **Extend shared owner** (new adapter in same crate) |
| Mac deployment scripts | `/Users/allen/production/mac-production-deployment/{project-ports.sh,shared-postgres.sh,project-action.sh,project-up.sh,project-status.sh}` + `port_allocator.py`/`compose-ports.py`/`project-db-overlay.py`/`generate-caddyfile.py` | 20-port block hashing, Caddy host rendering, shared-DB env mapping | Python/sh on target; not portable to cloud | jenkins-local / Mac | **Port faithfully into Forge Rust** |
| workspace-governance roster | `workspace-governance/projects.json`, `src/publish/fleet.rs` (`filter_eligible`) | 77-entry roster, `compose_ready` filtering (20 deployable) | Not a deployment concern; roster is just input | workspace-governance / Forge | **Reuse; no change** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Publish CLI/API | No new flags; `forge publish fleet [--dry-run]` renders the new `ssh`/`rsync`/`docker compose` plan; provider lifecycle unchanged |
| Adapter | New `RemoteComposeAdapter` (id e.g. `remote-compose`) alongside `JenkinsAdapter`; `JenkinsAdapter` stays compiled for rollback |
| Allocator / overlay / Caddy | New Rust modules carry existing Python semantics (fixtures as oracles); empty-DB case emits `services: {}` (fixing live `alethefy` break) |
| Persistence / journal | No new columns; queue/phase evidence unchanged |
| Target host | Read-only transition; `runtime/port-registry.json` + `platform/Caddyfile` still target-hosted but now Forge-written |
| Failure surface | Missing `docker`/`port-registry`/`shared-infra` surfaces as typed `deploy-target-unavailable`; subdomains never fabricate health |

## Capabilities

### New Capabilities

- `decoupled-remote-publish`: Forge-owned remote-compose fleet publish to a generic Docker target without target-hosted deployment scripts, including port allocation, DB overlay and Caddy rendering in Forge.

### Modified Capabilities

(none — `publish-commands`, `forge-independent-project-inventory-fleet` and `sibling-cwd-publish` keep their SHALLs; this package adds a new adapter mode behind the same `publish-commands` contract without changing their requirement text)

## Non-goals

- Publishing the fleet to GitHub Actions, Push idempotency changes, or inventing Docker images on Forge — not in this package.
- Removing the Mac script tree or secrets before the decoupled path is green.
- Changing the `sibling-cwd-publish` cwd-discovery contract or the `forge-project-inventory/0.1.0` fleet contract.

## Dependencies

- Archived: `forge-publish-plugin-orchestration` (publish engine), `forge-independent-project-inventory-fleet` (roster → `compose_ready`), `sibling-cwd-publish` (bare `forge publish` stays valid).
- `publish-commands` is the target contract (separate from `adapter-deployment`'s `forge-deploy-executor/0.1.0`); fleet uses the `PublishAdapter` lane, not the deploy-executor lane.

## Impact

Forge publish fleet execution path; new Rust modules for allocator/overlay/Caddy; no breaking CLI changes; no sibling checkout changes; no publish-domain vocabulary drift.
