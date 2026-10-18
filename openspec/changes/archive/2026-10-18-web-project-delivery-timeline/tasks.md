# Tasks: Unified read-only project delivery timeline

## BFS baseline

- [x] Confirm read owners, route/session conventions, event source fields, timestamp shapes, and existing Delivery UI/browser harness.
- [x] Define contract fixture and coverage semantics, including hosted observation and local Gate completeness limits.

## DFS requirement implementation

- [x] Add versioned timeline event/source DTO and deterministic bounded projection over existing stores.
- [x] Add session-gated read-only timeline route with project-ID gate, source isolation, and safe link/path projection.
- [x] Add Delivery page refresh flow and semantic responsive timeline states.
- [x] Add Rust API/projection contracts for auth, ordering, bounds, partial sources, status fidelity, and URL/path safety.
- [x] Add authenticated browser journey for empty/populated/partial/error, 390px layout, keyboard and announcements.

## BFS regression and completeness

- [x] Recheck all source owners and callers; confirm no write/provider/GitHub polling and no synthesized aggregate status.
- [x] Review compatibility, empty/error/session states, safe DOM rendering, and all event truncation notes.
- [x] Run focused source, browser, formatting and contract checks; review all added files and `git diff --check`.

## Verification

- [x] Run `node scripts/check-openspec-change-names.mjs` and `openspec validate --all --strict --no-interactive`.
- [x] Run `forge gate --dry-run`, then full bounded `forge gate`; record exact evidence in HANDOFF.
- [x] Archive without `--skip-specs`, promote canonical spec, commit implementation/archive, update pointer/HANDOFF, and create HANDOFF-only commit.
