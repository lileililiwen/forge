# Tasks: fleet-live-rollout

## 1. BFS — Baseline and impact coverage
- [x] 1.1 Map `cmd_publish_fleet` loop, `run_publish` registry writes, `Registry` (`rusqlite::Connection`, not `Sync`), adapter `RefCell` state; confirm main-thread-journal design needs no schema change. (Evidenced 2026-09-29: workers run with `registry=None`; main thread replays per-stage+summary rows via `replay_publish_journal` — no schema change, no shared connection.)
- [x] 1.2 Reproduce `--fleet-registry <path>` misroute with fixture; confirm timeout failures from live evidence (rsync 60s, cvunify 1800s). (Evidenced 2026-09-29: baseline refuses `inventory invalid: inventory contract must be ...` on a valid `projects.json`; fix verified to classify entries.)

## 2. DFS — Requirement-by-requirement implementation
- [x] 2.1 Add `--jobs N` (default 4): scoped threads, per-thread adapter+transport, serial journal + rendering on main thread; `--provider` branch stays sequential. Phased refinement: Phase A parallel Sync+Db, Phase B serial Prepare (registry converges), Phase C parallel Deploy — live evidence showed monolithic parallel Prepare is last-writer-wins on the shared registry/Caddyfile. (`FLEET_MAX_JOBS=32`, `validate_fleet_jobs`, `run_fleet_concurrent`, `run_fleet_entry_phases`, `replay_publish_journal`; 4 scheduler unit tests + 7 contract tests.)
- [x] 2.2 Add `PUBLISH_SYNC_TIMEOUT` (600s) to sync-stage commands via `CommandSpec::with_timeout`. (Both adapters; unit-pinned incl. never-renders assertion; probes keep 60s, deploy keeps 1800s.)
- [x] 2.3 Route explicit `--fleet-registry` through `legacy_inventory_snapshot` + fixture regression test. (Explicit flag now wins over `--inventory`; default-registry fallback preserved; 3 contract tests.)
- [x] 2.4 Execute ops runbook (legacy rm, secrets, sibling Dockerfile fixes) with recorded log. (Evidenced 2026-09-29: `hermexa-api` container + `hermexa-postgres-data` volume removed; secrets present for all 20 roster projects (file counts only); hermora `adduser`→`useradd` and hestia-lab `tdnf`→`apt-get wget` patched locally and synced to both Mac checkout paths; hestia-lab web needs nothing (nginx:alpine has busybox wget).)

## 3. BFS — Cross-surface regression and completeness
- [x] 3.1 Prove `--jobs 1` output identical to pre-change sequential (dry-run JSON diff). (Evidenced 2026-09-29: 21/21 docs identical modulo `elapsed_ms`; full-suite `jobs_one_dry_run_matches_default_dry_run` pins it.)
- [x] 3.2 Prove journal shapes, queue semantics, `--watch`, MCP/API untouched. (Evidenced: `replay_publish_journal` writes the exact `record_operation` shapes incl. summary row; queue row single + serial + roster-ordered; `cmd_fleet_online` untouched; full suite 81 groups / 1808 tests / 0 failed excluding one pre-existing env-bound listener test that fails on baseline too.)

## 4. Verification
- [x] 4.1 `cargo fmt/build/clippy/test` green; `git diff --check` pass. (Evidenced 2026-09-29: fmt clean on touched files — pre-existing drift in `gate/evidence.rs`, `portfolio/share/validation.rs`, `publish/fleet.rs`, gate/publish-queue tests preserved; clippy identical to baseline — same 10 locations, zero new; full suite 1808 passed / 0 failed.)
- [x] 4.2 Name preflight + `openspec validate --all --strict` 0 failed. (Evidenced 2026-09-29: preflight PASS; 51 passed, 0 failed; `git diff --check` PASS.)
- [ ] 4.3 Live `forge publish fleet --jobs 4` → 20/20 healthy, journal all done. (BLOCKED 2026-09-29: first attempt 4/20 `queue fleet-20260929T020916Z-c5365bdf` with monolithic path; then Mac Docker engine died — `no space left`, host 100% full; triage freed 1.3→9.4Gi via regenerable caches only and relaunched the app, but the engine never rebooted — no VM process, no new backend log writes. Phased code has no live evidence yet. Next action: operator recovers the engine — `docker version` must show a Server — then re-run the rollout; see HANDOFF ops log.)
