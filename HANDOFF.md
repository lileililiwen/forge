# Forge handoff

## Current state

`fleet-live-rollout` implemented, verified and archived on 2026-09-30 as
`2026-09-30-fleet-live-rollout`. Its requirements were promoted into
[openspec/specs/fleet-live-rollout/spec.md](openspec/specs/fleet-live-rollout/spec.md).
No active changes remain (no `current_spec` pointer).

- `forge publish fleet --jobs N` (default 4, `--jobs 1` sequential) now runs a
  phased scheduler: Phase A parallel `Sync`, Phase B serial `Db` + `Prepare`
  in roster order (the shared `production-postgres` and the global
  port-registry/Caddyfile renders converge), Phase C parallel `Deploy`; the
  shared platform-router reload is serialized by a new
  `CommandSpec::exclusive` flag in the transport. `--provider` stays
  sequential; journal rows and rendering stay on the main thread
  (`src/main.rs`, `src/publish/mod.rs`, `src/publish/remote_compose.rs`).
- Sync-stage transfers carry a 600s ceiling (`PUBLISH_SYNC_TIMEOUT`); probes
  keep 60s, deploy builds 1800s. `--fleet-registry <path>` routes through the
  legacy `projects.json` adapter instead of the inventory branch. The
  source-sync exclusion set is `.git/ node_modules/ target/ dist/ build/
  obj/ appendonlydir/ *.rdb` (was a wholesale `data/`, which dropped
  `rust-ecommerce` source).
- Target ops (recorded in the archived `design.md` D4 and `tasks.md` 2.4):
  secret provisioning, legacy container/volume removal, orphaned-network
  reclamation, and sibling build/healthcheck fixes (`hermora`, `hestia-lab`,
  `lexora`, `lore-mix`, `alltools-platform`, `opendockify`, `openaccounting`,
  `orphevia`, `reelio`, `rust-ecommerce`, `somodanote`, `trippify`).

## Verification (2026-09-30)

- Live: `forge publish fleet --jobs 4 --fleet-registry
  /home/paul/code/workspace-governance/projects.json` reached **20/20**
  (`fleet publish: 20/20 compose_ready succeeded`, `EXIT=0`); queue
  `fleet-20260930T102354Z-34c37313` and `forge deploy status --queue` shows
  all 20 rows `done`, each `fleet stages=4 healthy=true`. Run 6
  (`fleet-20260930T100950Z-c3e34795`) was 19/20 — the sole failure
  `alltools-platform` `prometheus` unhealthy (BusyBox `wget` behind the
  injected proxy; fixed with `-Y off`). The 2026-09-29 attempt had reached
  4/20 before the disk-full Docker engine death.
- `rustfmt --check` clean on the four touched files; `cargo clippy
  --workspace --all-targets` has no warning on any added line; `cargo test
  --workspace --all-targets --no-fail-fast` 2233 passed / 0 failed (`EXIT=0`);
  `git diff --check` PASS; `node scripts/check-openspec-change-names.mjs` PASS;
  `openspec validate --all --strict --no-interactive` 62 passed / 0 failed.
- App-internal crash loops that survive a successful deploy stage
  (`somodanote` EF migration, `orphevia` DI registration, `trippify` payment
  provider, `lore-mix` `DATABASE_URL`) are out of scope per the change's
  proposal non-goals and recorded in the archived `design.md` D4.
- No shared Gate Runtime is configured; no Gate pass is claimed.
