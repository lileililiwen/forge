# Tasks: fleet-live-rollout

## 1. BFS — Baseline and impact coverage
- [ ] 1.1 Map `cmd_publish_fleet` loop, `run_publish` registry writes, `Registry` (`rusqlite::Connection`, not `Sync`), adapter `RefCell` state; confirm main-thread-journal design needs no schema change.
- [ ] 1.2 Reproduce `--fleet-registry <path>` misroute with fixture; confirm timeout failures from live evidence (rsync 60s, cvunify 1800s).

## 2. DFS — Requirement-by-requirement implementation
- [ ] 2.1 Add `--jobs N` (default 4): scoped threads, per-thread adapter+transport, `run_publish(..., None)`, serial journal + rendering on main thread; `--provider` branch stays sequential.
- [ ] 2.2 Add `PUBLISH_SYNC_TIMEOUT` (600s) to sync-stage commands via `CommandSpec::with_timeout`.
- [ ] 2.3 Route explicit `--fleet-registry` through `legacy_inventory_snapshot` + fixture regression test.
- [ ] 2.4 Execute ops runbook (legacy rm, secrets, sibling Dockerfile fixes) with recorded log.

## 3. BFS — Cross-surface regression and completeness
- [ ] 3.1 Prove `--jobs 1` output identical to pre-change sequential (dry-run JSON diff).
- [ ] 3.2 Prove journal shapes, queue semantics, `--watch`, MCP/API untouched.

## 4. Verification
- [ ] 4.1 `cargo fmt/build/clippy/test` green; `git diff --check` pass.
- [ ] 4.2 Name preflight + `openspec validate --all --strict` 0 failed.
- [ ] 4.3 Live `forge publish fleet --jobs 4` → 20/20 healthy, journal all done.
