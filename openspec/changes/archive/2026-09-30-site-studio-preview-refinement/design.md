# Design: Site studio with live preview and scoped refinement

## 1. Implementation boundary

Repository `/home/paul/code/forge`, Rust 2021, MSRV 1.87.

- **New module `src/studio/`** for the bounded slice: `mod.rs` (exports +
  `STUDIO_CONTRACT_VERSION`), `spec.rs` (AppSpec parsing, validation,
  journal round-trip), `preview.rs` (preview session state machine,
  port reservation, bounded profile-runner spawn, timeout kill, log
  cap/redaction, idempotent stop), `state.rs` (per-project metadata
  persistence under `<project>/.forge/studio/`).
- **Extension points** are exactly: `src/lib.rs` (one new `pub mod
  studio;`), `src/core/mod.rs` (six additive typed errors and one
  additive `code()` mapping), `src/api/mod.rs` (five additive routes,
  one err_status arm, the `ApiConfig` in-memory live-preview map and the
  start/stop handlers that use it), `src/api/ui/{routes,render}.rs`
  (one read-only Studio page), `src/main.rs` (one new `Commands::Studio`
  branch). No changes to `src/process.rs`, `src/agent/`, `src/registry/`,
  `src/profile/`, `src/generate/`, or `src/publish/`.
- **No new schema, no new dependency.** The operations table is reused
  unchanged (it already carries `kind`, `detail`, `idempotency_key`,
  `revision`); `Cargo.toml` and `deny.toml` are unmodified. The shared
  `process::spawn_with_timeout` helper is reused for the profile
  runner — no new subprocess-adapter contract.
- **No new SQLite table, no new migration.** All Studio state is
  persisted either in the single session record at
  `<project>/.forge/studio/session.json` (which embeds the saved AppSpec)
  or as rows in the existing
  `operations` table (kind = `studio.spec.save`,
  `studio.preview.start`, `studio.preview.stop`, `studio.refine`).

## 2. Language and runtime

Rust (`cargo` workspace, edition 2021) for Forge, browser-native
JavaScript for the Studio page's static controls (no separate Node
build is introduced in Forge — the read-only Studio page embeds the
controls inline). The generated `react-web` app keeps its own
Node/Vite toolchain. Verification: `cargo fmt --all -- --check`,
focused `cargo test studio`, CLI contract tests with a fake profile
runner on a controlled `PATH`, cross-surface tests for CLI/API parity,
then Forge strict OpenSpec validation.

## 3. Ownership and shared code

Forge owns the product-level `forge.app.yaml` shape and preview
session. Workspace Governance remains owner of runtime deployment
templates; Forge only consumes the existing `react-web` profile for
this MVP and only for the spec/preview/refinement flow — the actual
scaffold is delegated to the existing deterministic generator
(`src/generate/`) on save. The preview runner argv is `ProcessRunner`'s
bounded array (`npm run dev` by default; the binary is overridable with
`FORGE_STUDIO_RUNNER_BIN` so a controlled stub stands in on a host with
no Node toolchain, exactly as `FORGE_GH_BIN` selects `gh`). The runner
receives the reserved port through `FORGE_STUDIO_PORT` and inherits the
parent `PATH` so the profile toolchain resolves; it is never invoked
through a shell.

The shared `src/process.rs::spawn_with_timeout` helper is reused for
the profile runner exactly as the GitHub CLI and deployer adapters
reuse it — same 30-second cap for short-lived probes, same 2 KiB
stderr cap with `[truncated]`, same kill-on-timeout semantics. The
preview module additionally owns a longer-lived monitor (1 MiB
stdout cap, 60-second startup window configurable via
`$FORGE_STUDIO_STARTUP_TIMEOUT_SECS` clamped to `1..=600`) that wraps
the shared helper.

## 4. Behavioral model

`forge.app.yaml` v1 contains:

- `schema_version` (`"1"`),
- `project_id` (kebab-case, validated),
- `name` (1..=120 chars),
- `profile` (closed set: today `react-web`; any other id is
  `studio-unsupported-profile`),
- ordered `pages` (each `{route, title, sections[]}`; `route` must
  start with `/`; `title` 1..=120 chars; `sections[]` ids must be
  unique within the spec),
- `theme` (`{preset, tokens}`; preset is closed set
  `{default, contrast, mono}`; tokens are an allowlisted key set),
- `acceptance_checks` (string list of allowed keyword predicates).

Validation rejects: unknown `schema_version` major, duplicate `route`,
duplicate section id, absolute or `..`-bearing paths, embedded shell
metacharacters, secret-shaped strings (`policy::redact_credentials`
token vocabulary is the deny list), unsupported profile, missing
project_id, project_id mismatch with the registered project, and
fields outside the closed vocabulary. Every refusal carries the exact
field path (`pages[2].sections[0].title`) so the operator can locate
it. **Validation is read-only** — it never mutates the project until
the operator explicitly confirms a `PUT` with the expected revision.

A `StudioSession` is the per-project state record, persisted at
`<project>/.forge/studio/session.json`:

- `contract` = `forge-studio-session/0.1.0`,
- `project_id`, `profile`, `app_schema_major` (always `1` today),
- `spec_revision` (monotonic `r<n>`; bumped on every successful
  `studio.spec.save`),
- `app_revision` (monotonic `r<n>`; bumped on every accepted
  `studio.refine` journal row; equals `spec_revision` until a
  refinement lands),
- `preview` (`{state, port, started_at, last_error_code}` where state
  ∈ `{none, starting, ready, failed, stopping, stopped}`).

A preview session is a **live child process**. Its exact owner depends
on the transport, because only a long-lived process can host a running
server:

- **API host.** The `forge api serve` process is long-lived, so it owns
  the live session in an in-memory map keyed by project id (the map is
  carried on `ApiConfig`). `POST /v1/projects/{id}/studio/preview` with
  `action: start` starts the runner, waits for readiness, persists the
  `ready` state, journals `studio.preview.start`, stores the live
  session, and returns the ready envelope; the session keeps running
  until `action: stop` (or server shutdown, which drops and kills it).
  A start request for a project that already has a live session first
  starts the new runner, then stops the previous one, so the previous
  preview stays reachable until the replacement reports `ready`.
- **CLI probe.** `forge studio preview <project> --action start` is a
  bounded readiness probe because the CLI exits immediately: it runs the
  same start, returns the `ready` envelope captured at readiness, then
  tears the preview down (killing only its own child) and persists the
  terminal `stopped` state. No detached process, PID file or port lease
  survives the CLI invocation. So an operator can exercise the probe
  without the API host, the CLI mirrors the revision-bound save at
  `forge studio spec <project> --confirm yes --expected-revision <rev>`
  (without `--confirm yes` the command stays read-only validation).

Both paths share the same Core functions (`studio::start_preview` and
`studio::stop_preview`) so the journal rows, revision binding and typed
failures are identical. Refinement creates a journal row but does
**not** spawn a new preview until the operator confirms a start; the
previous preview remains on its reserved port until the new process
reports `ready`.

Refinement is data-flow only in this cycle: the route validates the
request (revision matches the current `spec_revision` or
`app_revision`, every `selected_files` resolves inside the project
root, no shell metacharacters, no secret-shaped strings), records a
`studio.refine` journal row with the bounded detail and the new
`app_revision`, and returns the bumped revision. The follow-up cycle
wires this journal row to `agent::apply_transition` with a new
`SessionTransition::Refine` variant.

## 5. Contract and compatibility

Wire contracts (versioned):

- `forge-app-spec/0.1.0` (AppSpec payload),
- `forge-studio-session/0.1.0` (the persisted session record),
- `forge-studio-preview/0.1.0` (the bounded preview response envelope:
  `{contract, project_id, revision, state, port?, preview_url?,
  started_at, last_error_code?}`).

Routes:

| Method | Path | Verb | Confirmation gate |
| --- | --- | --- | --- |
| `GET`  | `/ui/studio/{id}` and `/ui/projects/{id}/studio` | `Route::UiStudioProject { id }` | bearer + origin (UI recheck) |
| `GET`  | `/v1/projects/{id}/studio/spec` | `Route::StudioSpec { id }` | bearer (project-scoped) |
| `POST` | `/v1/projects/{id}/studio/spec` | `Route::StudioSpecSave { id }` | bearer + `expected_revision` body field + `confirm=yes` body field |
| `GET`  | `/v1/projects/{id}/studio/preview` | `Route::StudioPreviewGet { id }` | bearer (project-scoped) |
| `POST` | `/v1/projects/{id}/studio/preview` | `Route::StudioPreviewPost { id }` | bearer + `confirm=yes` body field; `action: start|stop` |
| `POST` | `/v1/projects/{id}/studio/refine` | `Route::StudioRefine { id }` | bearer + `expected_revision` body field + `selected_files[]` body field |

Errors are typed `studio-invalid-spec`,
`studio-unsupported-profile`, `studio-port-unavailable`,
`studio-start-timeout`, `studio-revision-conflict`,
`studio-project-scope`. Existing `/ui`, `/v1`, and `forge.yaml`
payloads are unchanged; `err_status` adds one new arm mapping every
typed error to a stable HTTP code (`400` for invalid/refusal,
`409` for revision conflict, `503` for unavailable).

## 6. Failure and boundary policy

Missing project is `404 unknown-project` (re-used); unauthorized
project is `401 api-unauthorized` or `403 api-project-mismatch`
(re-used); invalid/stale spec is `400 studio-invalid-spec` /
`409 studio-revision-conflict`; unavailable agent or runtime is
`503 studio-start-timeout` or `503 studio-port-unavailable`, never
an empty success. Startup timeout kills the child process and records
`failed` with the typed error code (the exact stderr is
credential-redacted before it reaches the API/UI). Port collision
never kills an unrelated process — the Studio picks the next free
port in the documented range, refuses only when every candidate in
the range is taken, and surfaces `studio-port-unavailable` naming the
requested port. Path traversal and symlink escape are denied in the
spec validator before any filesystem mutation, and the CLI's
`--project <id>` resolves through the registry so an unregistered id
is `unknown-project` before Studio is touched. Preview logs are
capped at 1 MiB per session and secret-redacted before UI/API output.
Stopping a preview is idempotent — a `POST /preview` with
`action: stop` when no live session exists (or the persisted state is
already `stopped`) returns the same envelope and records exactly one
`studio.preview.stop` journal row with `state=stopped`. Starting a
preview with no saved session is `studio-invalid-spec`; a start that
fails to spawn, times out, or loses the port race persists
`state=failed` with the typed `last_error_code` before returning the
error, so a later `status` never reports a stale `ready`.

## 7. Verification oracle

Schema fixtures prove valid/invalid/boundary AppSpecs:

- `tests/fixtures/app-spec/valid/minimal.yaml` — smallest accepted
  spec (one page, one section, `default` theme, one acceptance check).
- `tests/fixtures/app-spec/valid/full.yaml` — every optional field
  populated, multi-page spec with 3 sections per page.
- `tests/fixtures/app-spec/invalid/duplicate-route.yaml`,
  `invalid/secret-shaped.yaml`, `invalid/path-escape.yaml`,
  `invalid/unsupported-profile.yaml`, `invalid/unknown-major.yaml`.

Unit tests (in `src/studio/`) prove deterministic render inputs,
revision checks, state transitions, and field-level rejection. Core
lifecycle tests (`tests/studio_preview_contract.rs`) use the in-process
`FakeRunner` to prove ready/start journaling, startup-failure persistence,
port-collision safety, and idempotent stop without any toolchain. CLI
(`tests/studio_cli_contract.rs`) and API
(`tests/studio_api_contract.rs`) tests drive a controlled runner stub
through `FORGE_STUDIO_RUNNER_BIN` to prove the bounded argv and the
transport-owned lifecycle; they skip when the stub host (`python3`) is
absent. Contract tests prove auth
and project isolation for every route, stale-write refusal, and
typed-refusal envelopes. Cross-surface tests prove the CLI and HTTP
transport return the same `forge-studio-preview/0.1.0` envelope for
the same inputs and that the same `studio.spec.save` row is recorded
on both transports. No test may claim arbitrary generated code is
sandboxed; local preview is explicitly operator-owned code execution.

## 8. Decision ledger

Resolved: initial target is static/client-rendered `react-web`; UI
extends existing Rust/maud API UI with a single read-only Studio
page; AppSpec is separate from infrastructure `forge.yaml`; preview
runs under the same user and project-root boundary as Forge's
existing local tooling; refinement routes through the same Core
service so the same `app_revision` semantics apply on CLI and HTTP.
Deferred: server-side profiles, identity/database features,
multi-user collaboration, public preview URLs, remote sandboxing, the
preview iframe proxy, the Studio interactive browser controls, and
the `studio.refine` → agent adapter runtime hook. Blockers: none for
this bounded local preview slice. The live `react-web` native
scaffold and the browser smoke remain open verification gaps to
close on a Node-equipped runner.