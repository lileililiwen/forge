# Design: fleet-live-rollout

## Context

Sequential fleet is O(sum of slowest builds). Live runs showed:
sync timeouts under contention (60s), one .NET build exceeding
1800s, legacy name/volume collisions, missing secrets, sibling
Dockerfile failures. All fixable; none belongs in the adapter.

## Decisions

### D1 — `--jobs N`, phased threads, main thread owns journal

**Chosen:** Rust `std::thread::scope`, N workers (default 4,
`--jobs 1` = today). Each worker builds its own
`RemoteComposeAdapter::from_env()` + `SubprocessTransport` (adapter
holds `RefCell` state: not shareable, cheap to construct) and
returns full `PublishReport`s over the scope boundary with
`registry = None` — no row escapes a worker. The main thread
replays every per-stage + summary journal row serially from the
returned reports (identical shapes to `run_publish`'s own writes;
only timestamps differ, as between any two runs) and renders all
output. `--provider` branch stays sequential (external path, rare).

**Phased refinement (live evidence 2026-09-29):** the first live
run exposed that Prepare renders *global* target state (the shared
`port-registry.json`, Caddyfile, index) from whatever the target
holds: N concurrent Prepares are last-writer-wins on those shared
files and each render misses its sibling projects. So the parallel
path runs in phases preserving per-project Sync → Db → Prepare →
Deploy order: Phase A runs Sync in parallel (per-project remote
dirs), Phase B runs Db + Prepare serially in roster order on the
main thread (the registry converges — each render sees every prior
project), Phase C runs Deploy in parallel (per-project
`forge-<id>` compose projects). Fail-fast stops scheduling new
projects at the phase boundary where the failure is observed; tail
slots never started are recorded as typed fail-fast skips. Dry-run
never takes the parallel path.

**Db serialisation (live evidence 2026-09-30):** Db is the
shared-Postgres ensure — its command
(`docker compose -p shared-postgres up -d production-postgres`) is
byte-identical for every project and operates on the one
`production-postgres` container. Running it in the parallel phase
made two workers recreate that container at once and the run
recorded `alethefy` and `crossify` as `shared PostgreSQL is
unavailable (exit 1)` with `removal of container … is already in
progress` / `Conflict. The container name … is already in use`.
Db therefore runs in serial Phase B, in roster order, immediately
before each project's Prepare. It is not safe to treat a
single-container shared resource as per-project idempotent work.

**Router reload serialisation (live evidence 2026-09-30):** the
Deploy stage ends by force-recreating the single platform router
container (`platform/compose.yml`, `production-router`) with the
converged Caddyfile. In Phase C two workers reached it at once and
one recorded `compose deploy rejected the request (exit 1)` with
`Container production-router Recreate … Conflict. The container
name "/…_production-router" is already in use` (`cvunify`,
`hermexa`). Fix: `CommandSpec` gains an `exclusive` flag; the
transport runs at most one exclusive command at a time across every
thread in the process, and the router reload sets it. Builds stay
parallel — only the shared reload is serialised — and the flag never
renders into dry-run plans, so the `--jobs 1` plan is unchanged.

**Deploy materialisation across phases (live evidence 2026-09-30):** the
first phased live run (queue `fleet-20260930T072642Z-da6ac884`) synced
and prepared all 20 projects but recorded every project as
`stages=3 healthy=false` with no Deploy report. Root cause: `Deploy`
builds its plan from state `Prepare` materialises on the adapter
(resolved Compose file + profiles, shared-DB overlay flag, env-file
probes, rendered router documents), and `run_publish` only calls
`materialize` when the request action *includes* `Prepare`. Phase C
runs `Deploy` alone on a fresh adapter, so `plan_deploy` failed closed
with `publish did not resolve a Compose file for this project`. Fix:
`run_fleet_entry_phases` materialises the `Prepare` inputs on the
fresh adapter whenever the phase subset contains `Deploy` but not
`Prepare` (`deploy_needs_prepare_materialization`). `materialize` only
reads the target and renders locally; the shared registry is written
exclusively by `Prepare`'s shipping step, which stays serial in Phase
B, so the added reads are side-effect free on the target and safe to
run across Phase C workers.

**Alternatives:** `rayon` (rejected: new dependency for a loop);
sharing `&Registry` via `Mutex` (rejected: `rusqlite::Connection`
is not `Sync`; also serializes the very writes we isolate);
monolithic parallel All per worker (rejected after live evidence:
unsound on the shared registry/Caddyfile).

### D2 — Timeouts

**Chosen:** `PUBLISH_SYNC_TIMEOUT = 600s` for sync-stage rsync
(`CommandSpec::with_timeout`, existing mechanism); 60s stays for
probes; 1800s stays for deploy builds.

### D3 — `--fleet-registry` repair

**Chosen:** route an explicit `--fleet-registry <path>` through
`legacy_inventory_snapshot` (one-branch fix in `cmd_publish_fleet`);
regression test with a fixture `projects.json`.

### D4 — Rollout runbook (target ops, recorded)

| Blocker (live evidence 2026-09-28) | Fix |
|---|---|
| `hermexa-api` legacy container holds name; `hermexa-postgres-data` held by `jenkins-hermexa` volume | `docker rm` exited legacy container + `docker volume rm` orphan (record names) |
| `hermora` Dockerfile `adduser` exit 127; missing `.env` | Fix Dockerfile useradd for image base; provision `.env` (done: keys present) |
| `hestia-lab` api healthcheck `wget` missing; `tdnf` absent in `aspnet:10.0` | Install via image-native manager (`apt-get` on Debian variant — verify base first) or replace probe with port-check the image supports |
| `hypora`/`hermora` `.env` absent | Provisioned 2026-09-28 (record keys, never values) |
| `cvunify` 1800s build | Retry with cache (now healthy per monitor) |
| `alltools-platform` 12-image build | Last solo or `--jobs` peer; pulls cached |

**Ops findings, run 5 (2026-09-30).** Three classes of target
blocker, none adapter-owned, all recorded:

- *Legacy containers holding static names.* The `forge-<project>`
  deploys set fixed `container_name`s in some compose files, so
  old containers from the bare-project / `jenkins-*` eras collide
  (`Conflict. The container name … is already in use`):
  `opendockify` (`jenkins-opendockify`), `aero-db`/`aero-redis`
  (`rust-ecommerce`), the exited `alltools-platform-*` set plus
  `consul-agent` (bare `alltools-platform`), and the
  `jenkins-chinago` / `jenkins-mortalect` sets. Removed
  (`docker rm -f <name>`) so the `forge-*` identity is free.
- *Docker address-pool exhaustion.* ~30 leftover project networks
  (many from `jenkins-*` runs) consumed every default predefined
  address pool; new `forge-<project>_default` networks then failed
  with `all predefined address pools have been fully subnetted`
  (e.g. `somodanote`). Removed the orphaned legacy networks
  (`rust-ecommerce_default`, `alltools-platform_alltools-network`,
  `jenkins-{chinago,mortalect,opendockify,alethefy,crossify,hermexa}_*`).
- *Registry/pull egress.* The host proxy on `7890` returns `502`
  for container-origin requests to Docker Hub (direct egress from a
  container works; the frontend image `docker/dockerfile:1` and
  `minio/minio:latest` could not be pulled). Two roster fixes avoid
  the pull without hiding the problem: `rust-ecommerce` drops the
  pinned `# syntax=docker/dockerfile:1` line (its `--mount=type=cache`
  steps build on the daemon's embedded frontend and the base image is
  already present), and `lore-mix`'s compose `minio/minio:latest` is
  satisfied from the already-present server image
  (`minio/minio:RELEASE.2024-10-13T13-34-11Z` retagged). A healthy
  proxy is still required for any cold pull.

**Ops findings, run 6 (2026-09-30).** One further target class, and
one out-of-scope observation:

- *BusyBox `wget` healthchecks behind the injected proxy.* With the
  engine injecting `HTTP_PROXY=http://host.docker.internal:7890`
  into every container, BusyBox `wget` (which ignores `no_proxy`)
  routed the in-container localhost probe through the proxy and got
  `502 Bad Gateway`; `alltools-platform`'s `prometheus` then
  reported unhealthy and its `rust-app` failed `depends_on`
  (`container forge-alltools-platform-prometheus-1 is unhealthy`).
  Fixed the same way as `lexora`: the `prometheus` and `grafana`
  healthchecks now pass `-Y off` (pinned in 2.4). Run 7 then reached
  20/20.
- *App-internal crash loops are out of scope.* Several `forge-*`
  deployments start but their application process exits or restarts
  (e.g. `somodanote` "pending model changes" EF migration,
  `orphevia` missing `BillingOptions` DI registration, `trippify`
  `Payment:Provider must not be 'local' in Staging`, `lore-mix`
  invalid `DATABASE_URL`). The proposal non-goals explicitly keep
  sibling app runtimes beyond build/deploy blockers out of scope;
  the rollout oracle is the per-stage command verdict (`healthy`),
  green for all 20.

### D5 — Verification oracle

- `cargo test` (scheduler unit incl. phased outcomes,
  `--fleet-registry` fixture test, sync-ceiling unit pins).
- `forge publish fleet --jobs 1 --dry-run` byte-identical to the
  pre-change sequential dry-run (modulo `elapsed_ms`/queue id).
- Live: one `forge publish fleet --jobs 4` → 20/20 `healthy=true`,
  `deploy status --queue` all `done`, containers `forge-*` up,
  verdicts cross-checked by `fleet-liveness-status` when built.
- **Green 2026-09-30.** After engine recovery and the run-4/5/6
  target ops (D4), run 6 (`queue fleet-20260930T100950Z-c3e34795`)
  reached 19/20 — the sole failure was `alltools-platform`
  (`prometheus` unhealthy, the BusyBox-`wget` proxy class in D4) —
  and run 7 (`queue fleet-20260930T102354Z-34c37313`) reached
  **20/20**: `fleet publish: 20/20 compose_ready succeeded`,
  `EXIT=0`, and `deploy status --queue` shows all 20 rows `done`,
  each `fleet stages=4 healthy=true`. The first live attempt
  (2026-09-29) had reached 4/20 before the disk-full engine death;
  the phased scheduler (D1) and the ops runbook (D4) closed the gap.
  App-internal crash loops are out of scope (D4, non-goal).

### D6 — Source-sync exclusions (live evidence 2026-09-30)

Two sibling build breakers traced to the sync exclusion set, not to
the projects:

- `data/` was excluded wholesale to keep mounted volume state off the
  target (`alltools-platform` binds `./data/redis`, whose
  dnsmasq-owned `appendonlydir/` makes rsync fail closed with exit
  23). But `rust-ecommerce` keeps *source* under `data/`
  (`data/admin_nav.json`, `include_str!` at build time) and under
  `src/i18n/data` (`include!` chunks), so the name-based exclusion
  dropped real source. Fix: exclude the concrete runtime artefacts
  (`appendonlydir/`, `*.rdb`) instead of the directory; a live rsync
  probe against the unreadable tree now exits 0 while `admin_nav.json`
  transfers.
- `.NET` `obj/` (intermediate output, including `project.assets.json`)
  was synced, and a Dockerfile's `COPY src/ …` then overwrote the
  fresh restore inside the image, failing `--no-restore` publish with
  `NETSDK1064` (`reelio`, `trippify`). Fix: exclude `obj/` (never
  source in any ecosystem in the roster). `bin/` is deliberately
  *not* excluded: `alltools-platform` has a source `rust/src/bin`.

`plan_sync` exclusion set is now `.git/ node_modules/ target/ dist/
build/ obj/ appendonlydir/ *.rdb`; the `sync_plan_is_mkdir_plus_rsync`
contract test pins the new patterns.

## Open Questions

None.
