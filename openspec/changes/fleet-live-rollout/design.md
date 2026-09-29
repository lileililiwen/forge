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
Deploy order: Phase A runs Sync+Db in parallel (per-project remote
dirs; Db is idempotent), Phase B runs Prepare serially in roster
order on the main thread (the registry converges — each render
sees every prior project), Phase C runs Deploy in parallel
(per-project `forge-<id>` compose projects). Fail-fast stops
scheduling new projects at the phase boundary where the failure
is observed; tail slots never started are recorded as typed
fail-fast skips. Dry-run never takes the parallel path.

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

### D5 — Verification oracle

- `cargo test` (scheduler unit incl. phased outcomes,
  `--fleet-registry` fixture test, sync-ceiling unit pins).
- `forge publish fleet --jobs 1 --dry-run` byte-identical to the
  pre-change sequential dry-run (modulo `elapsed_ms`/queue id).
- Live: one `forge publish fleet --jobs 4` → 20/20 `healthy=true`,
  `deploy status --queue` all `done`, containers `forge-*` up,
  verdicts cross-checked by `fleet-liveness-status` when built.
- **Blocked 2026-09-29:** the Mac Docker engine died mid-rollout
  (`no space left on device` writing its own `init.log`; host
  volume was 100% full) and has not rebooted after disk triage
  (regenerable caches cleared, 1.3 → 9.4Gi free; app relaunched,
  backend writes nothing new, no VM boot). First live attempt
  reached 4/20 with the monolithic parallel path; the phased
  refinement above is implemented and locally verified but has no
  live evidence yet. The engine recovery is an operator action
  (possible VM image damage from the disk-full crash); retry the
  rollout after `docker version` shows a Server.

## Open Questions

None.
