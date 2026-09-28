# Design: fleet-live-rollout

## Context

Sequential fleet is O(sum of slowest builds). Live runs showed:
sync timeouts under contention (60s), one .NET build exceeding
1800s, legacy name/volume collisions, missing secrets, sibling
Dockerfile failures. All fixable; none belongs in the adapter.

## Decisions

### D1 — `--jobs N`, threads own publish, main thread owns journal

**Chosen:** Rust `std::thread::scope`, N workers (default 4,
`--jobs 1` = today). Each worker builds its own
`RemoteComposeAdapter::from_env()` + `SubprocessTransport` (adapter
holds `RefCell` state: not shareable, cheap to construct) and calls
`run_publish(req, adapter, transport, None)` — no registry touch.
Workers return reports over the scope boundary; the main thread
writes per-stage/summary journal rows serially and renders output.
`--provider` branch stays sequential (external path, rare).

**Alternatives:** `rayon` (rejected: new dependency for a loop);
sharing `&Registry` via `Mutex` (rejected: `rusqlite::Connection`
is not `Sync`; also serializes the very writes we isolate).

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

- `cargo test` (scheduler unit, `--fleet-registry` fixture test).
- `forge publish fleet --jobs 4 --dry-run` shape unchanged.
- Live: one `forge publish fleet --jobs 4` → 20/20 `healthy=true`,
  `deploy status --queue` all `done`, containers `forge-*` up,
  verdicts cross-checked by `fleet-liveness-status` when built.

## Open Questions

None.
