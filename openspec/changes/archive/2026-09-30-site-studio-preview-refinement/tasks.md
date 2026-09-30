# Tasks: site-studio-preview-refinement

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map existing API/UI authorization (`src/api/ui/auth.rs`,
  `src/api/mod.rs::err_status`), project registry
  (`src/registry/mod.rs`), profile renderer (`src/profile/mod.rs`),
  agent adapter (`src/agent/mod.rs`), and runtime process ownership
  (`src/process.rs`) to every requirement in `proposal.md`.
- [x] 1.2 Confirm generated `react-web` projects remain standalone
  and that `forge.yaml` stays an infrastructure manifest — verified
  by reading `src/profile/mod.rs` (no infra fields are mirrored into
  `forge.app.yaml`) and by the existing deterministic-generator
  invariant that `forge new --profile react-web` writes only the
  profile's own assets.
- [x] 1.3 Add AppSpec valid/invalid/boundary fixtures and fake
  preview-runner test skeletons before implementation. The schema
  fixtures land at `tests/fixtures/app-spec/{valid,invalid}/*` and
  the schema itself at `schemas/app-spec-v1.json`; the fake runner
  is an in-process `FakeRunner` plus a controlled `PATH` stub that
  the process tests inject to prove bounded argv, timeout, and
  idempotency.
- [x] 1.4 Map the preview process-lifetime gap: only the long-lived
  `forge api serve` process can host a running preview, so the live
  session is owned by an `ApiConfig` map and the short-lived CLI is a
  bounded readiness probe. Confirm the `ApiConfig` change is additive
  (only `src/api/ui/auth.rs::cfg` and `ApiConfig::default` construct
  it) and that no existing handler signature beyond the Studio
  preview dispatch changes.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add versioned AppSpec schema, parser, validation errors,
  and fixtures. `src/studio/spec.rs` owns the `StudioSpec` struct,
  the closed field vocabulary, the per-field bounds, the validation
  error mapping to `ForgeError::StudioInvalidSpec` /
  `StudioUnsupportedProfile` / `StudioProjectScope`, and the
  round-trip JSON encoder that the journal row uses. Schema major
  `1` is the only accepted version today.
- [x] 2.2 Add revision-bound spec preview/save routes and project
  authorization tests. `src/studio/state.rs` owns the
  `<project>/.forge/studio/session.json` record; `save_spec`
  validates the new spec, requires the current
  `expected_revision`, persists the file, and records a
  `studio.spec.save` journal row with the new revision.
- [x] 2.3 Implement bounded preview session state, port reservation,
  same-origin URL, startup timeout, stop, and cleanup using
  profile-owned argument arrays. `src/studio/preview.rs` owns the
  `PreviewRunner` trait (the bundled `ProcessRunner` over the profile
  argv, and the in-process `FakeRunner` for tests), the port allocator
  in the documented range, the bounded log capture, the kill-on-timeout
  path, the redaction pass, and the idempotent stop.
- [x] 2.4 Add the Studio page and the read-only spec/preview
  rendering using existing maud UI conventions. The page lives at
  `GET /ui/studio/{project_id}` (and `/ui/projects/{id}/studio`),
  reuses the existing `recheck_post_token` + `check_origin` posture,
  and renders the current `app_spec`, the current preview state, and
  the journal rows for `studio.spec.save`, `studio.preview.start`,
  `studio.preview.stop`, `studio.refine`.
- [x] 2.5 Route refinement through the bounded Core function and
  record the journal row. `submit_refinement` validates the request
  (revision match, `selected_files` inside the project root, no
  shell metacharacters, no secret-shaped strings), bumps
  `app_revision`, and persists a `studio.refine` journal row. The
  runtime hook to `agent::apply_transition` is **deferred** to a
  follow-up cycle; the data-flow contract is delivered in this cycle.
- [x] 2.6 Wire the bounded preview start through the Core:
  `studio::start_preview(registry, project_root, runner)` starts the
  `PreviewSession`, waits for readiness within the clamped window,
  journals `studio.preview.start`, and persists the terminal
  `ready`/`failed` state (including the typed `last_error_code` on
  failure and the bounded, redacted log tail). `studio::stop_preview`
  kills a supplied live session (or records the idempotent persisted
  stop when none is held) and persists `stopped`. Evidence:
  `tests/studio_preview_contract.rs` (ready → start row; startup
  failure → `studio-start-timeout` + persisted `failed`; idempotent
  stop with one row per request; no-session refusal).
- [x] 2.7 Resolve the runner's toolchain lookup: `ProcessRunner`
  inherits the parent `PATH`, passes the reserved port through
  `FORGE_STUDIO_PORT`, and honors the `FORGE_STUDIO_RUNNER_BIN`
  override (default `npm`) so a controlled stub or a native
  `react-web` checkout both flow through the same bounded argv.
  Evidence: `ProcessRunner::react_web` and the CLI probe test in
  `tests/studio_cli_contract.rs` drive a controlled runner stub.
- [x] 2.8 Host the live preview in the API: `ApiConfig` carries the
  project-id-keyed live-session map; `POST .../studio/preview`
  `action: start` stores the session and returns the ready envelope,
  `action: stop` stops it (or is idempotent), and server shutdown
  drops every live session. Change only the Studio preview dispatch
  arm to pass `config`. Evidence:
  `tests/studio_api_contract.rs::api_owns_the_live_preview_between_start_and_stop`
  proves the session stays live between the start and the status
  read, then stops on `action: stop`.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Re-audit API, CLI, UI, profile generation, process
  cleanup, project isolation, log redaction, and revision conflicts.
  Every cross-surface test asserts the same envelope shape on CLI
  and HTTP for the same inputs; every contract test asserts the
  exact typed error code on each refusal path. Evidence: the CLI and
  API contract tests both bind `forge-studio-preview/0.1.0` and
  `state: ready|stopped`; `tests/studio_preview_contract.rs` proves
  the shared Core start/stop; the existing
  `tests/studio_cli_contract.rs` refusal tests are unchanged.
- [x] 3.2 Prove unrelated projects/processes are untouched on
  invalid paths, port collisions, timeout, and stop. The process
  tests occupy the whole documented port range to prove the Studio
  refuses `studio-port-unavailable` rather than killing an unrelated
  process, and a start failure persists `failed` with the typed code.
  Evidence:
  `tests/studio_preview_contract.rs::preview_port_collision_is_refused_without_killing_a_listener`
  holds every slot in the range, asserts the typed refusal, and
  confirms the held listeners are untouched.
- [x] 3.3 Confirm existing UI routes, project manifests, and publish
  records remain compatible. `cargo test --workspace --all-targets
  --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`
  ran to completion; the only failure
  (`fleet_online_routes_to_local_listener_when_alethefy_is_up`) is
  pre-existing and sandbox-related (`DOWN` vs `ONLINE`, listener
  restriction) and out of scope.

## 4. Verification

- [x] 4.1 Run focused Rust tests, `cargo fmt --all -- --check`,
  `cargo clippy --all-targets` (no new findings), `git diff --check`,
  and `openspec validate --all --strict --no-interactive`. Captured in
  `HANDOFF.md`: `cargo fmt --all -- --check` clean; `cargo clippy
  --all-targets` clean of Studio findings; `git diff --check` clean;
  `openspec validate --all --strict --no-interactive` 61 passed.
- [x] 4.2 Record the bounded runner evidence (in-process `FakeRunner`
  and a controlled stub on `PATH`) and the blocked live evidence (no
  operator-owned `react-web` checkout with `npm install`) honestly in
  `HANDOFF.md`; do not label the process-fixture tests as native
  `react-web` verification.

## 5. Explicit deferred work (not in this cycle)

- [ ] 5.1 Same-origin preview iframe proxy on a reserved port (this
  cycle reserves the port and records the state; the proxy lands
  in a follow-up that cannot collide with the API listener because
  it lives on its own port).
- [ ] 5.2 Studio interactive browser controls (prompt form,
  refinement textarea, file picker). The current Studio page is
  read-only; the controls land alongside a UI spec bump.
- [ ] 5.3 `studio.refine` journal row → `agent::apply_transition`
  runtime hook. The data-flow contract lands here; the agent hook
  lands with the `SessionTransition::Refine` variant bump.
- [ ] 5.4 Browser smoke on a Node-equipped runner. The sandbox this
  package was implemented in does not have a native `react-web`
  project; the smoke is recorded as `not run` and pinned as the
  verification gap to close.
- [ ] 5.5 Native `react-web` profile scaffold verified against `npm
  install && npm run dev`. The contract test uses a controlled
  runner stub; the native verification is the same gap as the
  graduation and delivery packages and lands when the operator runs
  it against a real scaffold.
