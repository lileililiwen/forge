# Proposal: fleet live rollout (20/20 green + operable fleet publish)

## Why

`decoupled-remote-publish` delivered the code lane (default
`remote-compose`, live canaries green) but its full-fleet acceptance
moved here: a sequential `forge publish fleet` is too slow for 20+
projects (12-image builds, multi-GB rsyncs, one 1800s-timeout build
stalls everything behind it), and live evidence exposed a backlog of
per-project blockers no adapter code can fix (missing target secrets,
orphaned legacy `jenkins-*` containers/volumes, project Dockerfiles
that fail to build, oversized sync trees). Operating the fleet today
also requires shell hacks (`xargs -P8`, hiding compose files) that no
other operator or AI can discover — the product must do this natively.

## What Changes

- `forge publish fleet --jobs N` (default 4): concurrent per-project
  publish with serialized target-registry writes and serialized
  journal rows; `--jobs 1` preserves today's sequential behavior.
  Fail-fast still stops scheduling new projects.
- Sync timeout fit for purpose (600s ceiling for rsync; keeps 60s for
  fast probes), so big trees don't die under contention.
- Rollout runbook as code: provision missing target secrets
  (`hermora`, `hypora` done; same pattern for the rest), remove
  orphaned legacy containers/volumes blocking static names
  (`hermexa-api` class), fix sibling build breakers (`hermora`
  `adduser`, `hestia-lab` healthcheck tooling — `tdnf` absent in
  `aspnet:10.0`), then 20/20 fleet green.
- Fix `--fleet-registry <path>` (currently falls into the inventory
  branch and rejects `projects.json` shape) or remove it and keep one
  roster path.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `fleet-live-rollout` (Forge Rust + target ops) | `forge publish fleet --jobs 4` goes 20/20 green on the Mac with no shell hacks | Forge Rust; target owns Docker + file tree | Existing `forge-publish/0.1.0` + journal contracts, unchanged | `decoupled-remote-publish` | Live 20/20 + `deploy status` all-done |
| `fleet-liveness-status` (already proposed) | CLI online verdicts | Forge Rust | `forge-fleet-liveness/0.1.0` | this package (needs green fleet to verify against) | Live verdicts |
| `portal-web-ui` (already proposed) | Browser list/manage | Forge Rust | HTML over API | liveness | HTTP tests + capture |

One outcome (operable, green fleet), one owner, one oracle.
Liveness and portal stay separate consumers.

## Sibling Reconnaissance

| Candidate | Decision |
|---|---|
| `run_publish` + `SubprocessTransport` (`src/publish/mod.rs`) | **Extend shared owner** (per-thread adapter instances; registry writes stay on main thread) |
| `cmd_publish_fleet` loop (`src/main.rs`) | **Extend** (`--jobs`, thread pool, serialized journal) |
| Legacy `jenkins-*` containers/volumes on Mac | **Ops cleanup** (dev env, no users; record removals) |
| Sibling Dockerfiles (`hermora`, `hestiaLab`) | **Patch upstream** (one-line build fixes, owned by those projects) |
| `xargs`/hide-file hacks | **Delete** (replaced by `--jobs`) |

## BFS Impact Map

| Surface | Impact |
|---|---|
| CLI | +`--jobs N` on `publish fleet`; nothing else changes |
| Journal | Same rows, written serially; queue id unchanged |
| Target | Same commands; registry read-modify-write serialized per project (unchanged protocol, just no interleave) |
| Contracts | None new, none changed |
| Tests | Unit (job scheduling, serialization) + contract (`--jobs 1` byte-identical to today) + live 20/20 |

## Non-goals

- Liveness verdicts, web UI, alerting (separate packages).
- Cloud migration (adapter already env-only; no live cloud target in scope).
- Fixing sibling app runtimes beyond build/deploy blockers.
