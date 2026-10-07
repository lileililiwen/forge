# Design: Browser-executable provider publish

## 1. Implementation boundary

- **Repository / project:** this Forge repository (`/home/paul/code/forge`), the
  `forge` crate. No sibling project is touched.
- **Modules changed:**
  - `src/api/mod.rs` — add the `AdminProjectPublishPlan` / `AdminProjectPublish`
    route variants, the two router arms, and the short-circuit, permission,
    dispatch and authorize list entries; export the two route consts through
    `admin`.
  - `src/api/admin.rs` — add the two route consts, a publish target resolver
    (project id gate → provider env → provider config → provider selection →
    git revision), path-free plan/result views, a scrubbed typed failure mapper
    and the `record_publish_phase` journal write.
  - `src/api/command_catalog.rs` — flip the bare `publish` row to a `web_exec`
    row via the existing builder, extend `IMPLEMENTED_WEB_ROUTES`, update the
    pinned `web` tuple / `executable_ids` / execution-parameter tests.
  - `tests/forge_web_command_catalog_contract.rs` — add the publish apply route
    to the web allowlist and `publish` to the `web_ids` set.
  - `frontend/app.js` — expected to need **no** change: `buildActionControl`
    already renders any `execution` row generically, and a zero-parameter row
    renders a preview → confirm → run control with no inputs.
- **Modules reused unchanged:** `src/publish/providers.rs`
  (`load_config`/`select_provider`/`invoke_provider`, `PublishProviderRequest`,
  `ProviderEntry`, `ProviderOperation`, `PUBLISH_PROVIDER_CONTRACT`,
  `validate_revision`, `compose_project_name`) and
  `src/registry/mod.rs` (`Registry::record_publish_phase`,
  `PublishPhaseEvidence`).
- **Must NOT change:** the CLI `forge publish` dispatch, the
  `forge-publish-provider/0.1.0` contract, the `src/delivery` adapters, the
  `CONTRACT_VERSION` of the catalog or API, or the deploy/release/authoring
  routes.

## 2. Language and runtime

- Rust 2021, toolchain floor `rustc 1.87` (per `Cargo.toml` `rust-version`).
- Build: `cargo build`. Test: `cargo test` (and `cargo test --bin forge` for the
  in-source catalog tests). Format: `cargo fmt`.
- Frontend: plain ES modules under `frontend/`, served by `forge web serve`; no
  bundler, no new dependency.
- Target platform: Linux loopback server; the route resolves the git revision by
  shelling out to `git` with a fixed argv and invokes the configured provider
  executable with a fixed argv exactly as the CLI already does.

## 3. Ownership and shared code

- `publish::providers` owns provider semantics; the admin route is a thin
  consumer, exactly as `cmd_publish_provider` and `handle_github_push` are. The
  new code MUST call the same `select_provider` + `invoke_provider` and MUST NOT
  reimplement provider dispatch, argv construction, timeout or response
  validation.
- `src/api/admin.rs` owns the session-gated confirm→digest discipline shared by
  authoring, deploy and release; publish joins that owner rather than opening a
  parallel gate. The digest is computed over a **path-free** canonical
  descriptor (project id + provider id + resolved revision), mirroring
  `authoring_digest`.
- The `publish` journal row is written with `Registry::record_publish_phase`
  exactly as `cmd_publish_provider` writes it, so the fleet `published`
  projection (`src/api/fleet.rs::latest_publishes`, `kind IN
  ('publish','publish.github')`) reads the same state and phase columns.
- No shared `/lib` or sibling extraction; project-local extension of existing
  owners only (per the proposal reconnaissance table).

## 4. Behavioral model

Actor: the single authenticated global admin (opaque `forge_admin_session`
cookie). Scope: exactly one managed project id; the provider and revision are
server-resolved.

**Server-side resolution** (shared by both routes, in order):
1. `deploy_id_gate(db_path, id)` — `validate_project_id` → 400
   `admin-invalid-project-id` (no echo); `registry.inspect(id)` → 404
   `admin-project-unmanaged`; yields the project directory.
2. `FORGE_PUBLISH_PROVIDER` (trimmed, non-empty) → provider id; unset/blank → 409
   `admin-prerequisite` naming the variable.
3. Provider configuration path: `FORGE_PUBLISH_PROVIDER_CONFIG` when set, else
   `<project_dir>/.forge/providers.yaml`. `publish::providers::load_config` →
   `ProviderConfig`; a missing/invalid file → 409 `admin-prerequisite` with a
   static message (the absolute path is never echoed).
4. `publish::providers::select_provider(&config, &provider_id)` → `ProviderEntry`;
   unknown/disabled → 409 `admin-prerequisite` (the provider id value is never
   echoed).
5. Commit revision: `git -C <project_dir> rev-parse HEAD`, then
   `validate_revision`; missing/non-40-hex → 409 `admin-prerequisite` (a publish
   needs a committed revision to bind). The directory is used only to locate the
   manifest/config and is never serialized.

**`GET /v1/admin/projects/{id}/publish/plan`** (read-only):
1. `guarded` session gate → 401 if invalid.
2. Server-side resolution (above) → `{project_dir, provider_id, entry, revision}`.
3. Return 200 with a path-free plan view (`action`, `project_id`, `provider`,
   `operation: "publish"`, `revision`, `note`) and `plan_digest =
   authoring_digest({"project_id": id, "provider": provider_id, "revision":
   revision})`. No provider is invoked, no journal row is written.

**`POST /v1/admin/projects/{id}/publish`** (confirm-gated apply):
1. `is_json` → 415 `admin-content-type-required`.
2. `guarded` session gate → 401.
3. Server-side resolution → digest, both before any provider call.
4. If `confirm` not true → 200 `{ preview, plan_digest, confirmation: {
   requires: ["confirm","plan_digest"] } }`; **no provider call, no journal**.
5. If `confirm` true and `plan_digest` mismatched → 409 `admin-digest-mismatch`
   with a fresh preview + digest; **no provider call, no journal**.
6. If `confirm` true and matching → build `PublishProviderRequest` (fixed
   contract, `ProviderOperation::Publish`, server provider id, server revision,
   `operation_id = "publish-<id>-<sha12>"`, `folder = project_dir`) and
   `invoke_provider(&entry, &request, &project_dir)`:
   - On `Err` → `record_publish_phase(id, "failed", PublishPhaseEvidence::new()
     .revision(&revision), Some(<scrubbed reason>))` and return **503
     `publish-provider-unavailable`** with the scrubbed reason. Never success.
   - On `Ok(response)` → derive `phase_revision` (response echo, else request
     revision) and `container_identity` (response echo, else
     `compose_project_name`), then `record_publish_phase(id, &response.status,
     evidence, Some("provider=<p> revision=<r> health=<h> build=<..> run=<..>"))`,
     and return **202** with `{project_id, operation_id, provider, operation,
     revision, status, health, healthy, build_status, run_status,
     container_identity, evidence, recovery}`, scrubbed of the project dir and
     the provider executable path. `healthy` is true only when the provider
     reported `status == "done"` and `health == "healthy"`, so an unhealthy
     provider response is reported as `healthy:false` and its own `status`, never
     as a success.

State transitions: one `publish` row in the existing `operations` table per
confirmed run, carrying the provider-reported state and the additive phase
columns; a pre-confirmation or mismatched-digest request writes nothing. No new
on-disk state, no schema change.

## 5. Contract and compatibility

- New route consts added to `IMPLEMENTED_WEB_ROUTES`:
  `ROUTE_ADMIN_PUBLISH_PLAN = "GET /v1/admin/projects/{id}/publish/plan"`,
  `ROUTE_ADMIN_PUBLISH = "POST /v1/admin/projects/{id}/publish"`.
- Request body (`POST`): `{ "confirm": bool, "plan_digest": string? }`. Only
  these keys are read; anything else is ignored, never treated as a provider id,
  revision, path/argv/shell, host, SSH target or credential. The plan route takes
  no parameters.
- Catalog `publish` row: `availability: web`, `risk: remote_write`, `route:
  "POST /v1/admin/projects/{id}/publish"`, and an `execution` block `{ route,
  method: "POST", risk: "remote_write", confirm_required: true, digest_bound:
  true, parameters: [] }`. It is the one executable row with no browser input,
  because every input (project id, provider, revision) is server-resolved.
- Response shapes are additive; the CLI publish output and every existing
  response are unchanged. No schema/migration.
- Validation ownership: id/JSON/session/prerequisite/digest gates in `admin.rs`;
  provider-domain validation (contract shape, revision, secret scan, timeout)
  stays in `publish::providers` and surfaces as its typed errors.

## 6. Failure and boundary policy

| Case | Behavior |
|---|---|
| No / invalid session cookie | 401, no provider call |
| Non-JSON POST | 415 |
| Path-bearing or empty project id | 400 `admin-invalid-project-id`, no echo |
| Unknown / unmanaged project id | 404 `admin-project-unmanaged` |
| `FORGE_PUBLISH_PROVIDER` unset / blank | 409 `admin-prerequisite`, names the variable, no value |
| Provider config missing / invalid | 409 `admin-prerequisite`, no path echo |
| Provider unknown / disabled | 409 `admin-prerequisite`, no id echo |
| No resolvable 40-hex revision | 409 `admin-prerequisite` |
| `confirm` absent/false | 200 preview + digest, **no provider call, no journal** |
| Digest mismatch | 409 `admin-digest-mismatch`, fresh digest, **no write** |
| Provider spawn failure / timeout / non-zero exit / invalid or secret-bearing response | 503 `publish-provider-unavailable`, journaled `failed`, never success |
| Provider reports non-healthy status | 202 with the provider's real `status`/`health`/evidence and `healthy:false`, journaled with that state |
| Any absolute path / provider binary / secret in a response | scrubbed before return (project dir + provider executable + config path) |

No case is silently swallowed; a failed or unhealthy provider outcome is
reported as failed, consistent with "partial external outcomes remain partial."

## 7. Verification oracle

- **New file `tests/forge_web_project_publish_contract.rs`** (mirrors
  `forge_web_project_release_contract.rs`), asserting against a temp registry
  seeded with one managed git project whose `.forge/providers.yaml` names a
  hermetic `#!/bin/sh` stub provider that drains stdin and emits a valid
  `PublishProviderResponse`:
  1. `GET …/publish/plan` and `POST …/publish` without a session → 401; POST with
     non-JSON → 415; the stub provider log is absent (no invocation) and the
     registry has no `publish` row.
  2. Path-bearing id → 400 with no path echo; unmanaged id → 404.
  3. `FORGE_PUBLISH_PROVIDER` unset → 409 `admin-prerequisite`; a config path
     pointing at a missing file → 409; neither echoes a value or path.
  4. `GET …/publish/plan` with a configured stub provider → 200 with a path-free
     plan (`provider`, 40-hex `revision`) and a 64-hex `plan_digest`; the stub log
     is absent (no invocation) and no `publish` row exists.
  5. `POST …/publish` without `confirm` → 200 with a 64-hex `plan_digest` and a
     path-free preview; no invocation, no row.
  6. `POST …/publish` with `confirm:true` and a wrong digest → 409
     `admin-digest-mismatch`, fresh digest, no invocation, no row.
  7. Confirmed run with a matching digest **invokes the stub provider once**:
     assert 202, the stub received the request in a log, a `publish` `operations`
     row for the project carries `state:"done"` and the phase columns, `healthy`
     is true, and the response carries no absolute project path and no provider
     binary path.
  8. Honest-failure case: a stub provider that exits non-zero → 503 with a typed
     error, a `publish` row with `state:"failed"`, and no success claim.
  9. Catalog: the bare `publish` row is `web` with an `execution` block naming the
     admin route and an empty typed-parameter list; both routes are listed in
     `IMPLEMENTED_WEB_ROUTES`.
- **In-source catalog tests** (`src/api/command_catalog.rs`): update the pinned
  `web` tuple and `executable_ids` (7 → 8, adding `publish`); the
  `catalog_has_no_integrity_problems`, `catalog_covers_every_clap_path` (count
  stays 227 — dispositions change, not the command set) and
  `web_rows_only_point_at_implemented_routes` tests must pass.
- **Existing contract files** must stay green:
  `forge_web_command_catalog_contract`, `forge_web_command_execution_contract`,
  `forge_web_project_deployment_contract`, `forge_web_project_release_contract`,
  `forge_web_project_actions_contract`, `forge_web_project_workbench_contract`,
  `forge_admin_api_contract`, `forge_web_publish_fleet_contract`,
  `publish_contract`, `portal_ui_contract`.
- A task box is checked only with the command output for its assertion.

## 8. Decision ledger

- **Resolved:** provider publish is exposed on the admin surface reusing
  `publish::providers` unchanged (extend, not reimplement); plan is a read route
  and apply is a confirm→digest `web_exec`; the provider id and revision are
  resolved server-side and only their canonical values are bound into the digest.
- **Resolved:** the browser supplies no parameters at all for the bare publish —
  the executable row's parameter list is empty — because the project id, provider
  and revision are server-side; the apply body is reduced to `confirm` and
  `plan_digest`.
- **Resolved:** the journal is one `publish` row written with
  `Registry::record_publish_phase`, carrying the provider-reported state and the
  additive phase evidence, exactly as `cmd_publish_provider` writes it. This is
  chosen over `run_with_operation` because `run_with_operation` finalizes with a
  generic `done` detail that would discard the provider `health`/phase fields the
  fleet `published` projection consumes; the provider's own state is the honest
  terminal state.
- **Resolved (split):** delivery with health-gated promotion is
  `forge-web-project-delivery`, which depends on this change; this change ships
  provider publish only.
- **Deferred (out of scope):** OpenPanel `delivery preflight`/`stage`/`promote`/
  `hermora-retry`, `publish.provider.*` lifecycle mutations, provider fleet
  publish, and a browser-side provider/revision selector; all stay honest
  `provider_required`/`not_yet_web`.
- **Blockers:** none. The provider contract, the confirm→digest gate pattern,
  the catalog builder, the route allowlist and the `publish` journal write all
  exist; no unresolved material decision.
