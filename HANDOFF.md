current_spec: fleet-live-rollout

# Forge handoff

## Current state

`site-studio-preview-refinement` implemented, verified and archived on
2026-09-30 as `2026-09-30-site-studio-preview-refinement`; its three
requirements (versioned AppSpec, bounded live preview, scoped refinement)
were promoted into [openspec/specs/site-studio-preview-refinement/spec.md](openspec/specs/site-studio-preview-refinement/spec.md).

- Domain `src/studio/{mod,spec,state,preview}.rs`: closed AppSpec parser and
  validator, persisted session record at
  `<project>/.forge/studio/session.json`, and a bounded preview lifecycle.
  `studio::start_preview` / `studio::stop_preview` are the shared Core entry
  points. `ProcessRunner::react_web` runs the profile argv (default
  `npm run dev`, override `FORGE_STUDIO_RUNNER_BIN`, reserved port via
  `FORGE_STUDIO_PORT`, inherited `PATH`, no shell).
- The long-lived API owns the live session in an `ApiConfig` project-keyed
  map; the short-lived CLI is a bounded start → ready → stop probe that
  leaves no detached process. CLI parity adds
  `forge studio spec <project> --confirm yes --expected-revision <rev>`.
- Routes: `/v1/projects/{id}/studio/{spec,preview,refine}` and
  `GET /ui/studio/{id}`. Six additive typed errors
  (`studio-invalid-spec`, `studio-unsupported-profile`,
  `studio-port-unavailable`, `studio-start-timeout`,
  `studio-revision-conflict`, `studio-project-scope`). No new schema,
  dependency, migration, or SQLite table.

## Verification (2026-09-30)

- `cargo fmt --all -- --check`, `cargo clippy --all-targets` (no Studio
  findings), `git diff --check`, and
  `openspec validate --all --strict --no-interactive` (61 passed): clean.
- Tests: `--lib -- studio` 31; `tests/studio_preview_contract` 6;
  `tests/studio_api_contract` 2; `tests/studio_cli_contract` 13.
- `cargo test --workspace --all-targets --no-fail-fast -- --skip
  rust_scaffold_builds_and_tests_with_native_toolchain`: only the
  pre-existing sandbox failure
  `fleet_online_routes_to_local_listener_when_alethefy_is_up` (DOWN vs
  ONLINE listener restriction) remains.
- Blocked / not claimed: native `react-web` scaffold and browser smoke need
  a Node-equipped runner (`tasks.md` §5.1–§5.5); `cargo deny check` needs
  network. The preview oracle is the in-process `FakeRunner` for Core
  lifecycle tests and a controlled `python3` runner stub for the CLI/API
  contract tests.
- Pointer: `site-studio-preview-refinement` archived. The only remaining
  active change is **`fleet-live-rollout`** (10/11, blocked on Mac Docker
  engine recovery). No shared Gate Runtime is configured; no Gate pass is
  claimed.
