# Design: Browser-executable project deploy

## 1. Implementation boundary

- **Repository / project:** this Forge repository (`/home/paul/code/forge`), the
  `forge` crate. No sibling project is touched.
- **Modules changed:**
  - `src/api/mod.rs` — add two router arms and group/CORS matching for the admin
    deploy routes; export the two route consts.
  - `src/api/admin.rs` — add a deploy kind to the existing admin gate
    (`Authoring`-style enum, descriptor, digest, write) and delegate to
    `deploy::engine`.
  - `src/api/command_catalog.rs` — add `deploy.plan`/`deploy.apply` rows via the
    existing builders, extend `IMPLEMENTED_WEB_ROUTES`, update the pinned
    `executable_ids` test.
  - `frontend/app.js` — expected to need **no** change: it already renders any
    `execution` row generically. If the deploy `target` parameter needs a
    project-derived candidate list, that is a follow-up; the base control renders
    a typed text field.
- **Modules reused unchanged:** `src/deploy/mod.rs` (types, `DeployConfig`,
  `DeployAdapterConfig::from_env`, `load_config`), `src/deploy/engine.rs`
  (`prepare_deploy`, `apply_deploy`, `invoke_adapter`), the registry `operations`
  journaling used by `run_with_operation`.
- **Must NOT change:** the bearer `handle_apply_deployment`
  (`src/api/mod.rs:3080`) and its `POST /v1/projects/{id}/deployments` route; the
  portfolio-share pipeline (`src/api/delivery.rs`); any Core deploy semantics;
  `CONTRACT_VERSION` of the catalog or API; the CLI dispatch.

## 2. Language and runtime

- Rust 2021, toolchain floor `rustc 1.87` (per `Cargo.toml` `rust-version`).
- Build: `cargo build`. Test: `cargo test` (and `cargo test --bin forge` for the
  in-source catalog tests). Format: `cargo fmt`.
- Frontend: plain ES modules under `frontend/`, served by `forge web serve`; no
  bundler, no new dependency.
- Target platform: Linux loopback server; the API and web listeners run in the
  same process family as today.

## 3. Ownership and shared code

- `deploy::engine` owns deploy semantics; the admin route is a thin consumer,
  exactly as `handle_apply_deployment` is. The new code must call the same
  `prepare_deploy` / `apply_deploy` and must not reimplement plan/apply.
- `src/api/admin.rs` owns the session-gated confirm→digest discipline shared by
  authoring; deploy joins that owner rather than opening a parallel gate. The
  digest is computed over a **path-free** canonical descriptor (project id +
  normalized target), mirroring `authoring_digest`.
- No shared `/lib` or sibling extraction; project-local extension of existing
  owners only (per the proposal reconnaissance table).

## 4. Behavioral model

Actor: the single authenticated global admin (opaque `forge_admin_session`
cookie). Scope: exactly one managed project id.

**`GET /v1/admin/projects/{id}/deploy/plan`** (read-only):
1. Session gate (`guarded`) → 401 if invalid.
2. `validate_project_id(id)` → 400 `admin-invalid-project-id` (rejects any id
   containing a path); `registry.inspect(id)` → 404 `admin-project-unmanaged`.
3. Resolve `project_dir = PathBuf::from(registry row.path)` **server-side**.
4. `deploy::engine::load_config(&project_dir)` → `(Manifest, DeployConfig)`.
5. `prepare_deploy(project_dir, &manifest, &config, &DeployRequest { project_id,
   target, confirm:false, dry_run:true })` → `DeployPlan` (invokes no adapter,
   writes nothing).
6. Return 200 with a path-free plan view: target name/kind, source_revision,
   artifact hash/size, health check, `ready`. Absolute paths and the adapter
   binary are stripped (`redact`/scrub as in `delivery.rs`).

**`POST /v1/admin/projects/{id}/deploy`** (confirm-gated apply):
1. `is_json` → 415 `application/json` required.
2. Session gate → 401.
3. id gate → 400 / 404 (before any descriptor/digest, as in `authoring_write`).
4. Build descriptor `{ project_id, target }` (target empty ⇒ server resolves
   `config.default_target`); `plan_digest = sha256(canonical descriptor)`.
5. If `confirm` not true → 200 `{ preview: <plan view>, plan_digest,
   confirmation: { requires: ["confirm","plan_digest"] } }`; **no write, no
   adapter call**.
6. If `confirm` true but `plan_digest` mismatched → 409 `admin-digest-mismatch`
   with a fresh preview + digest; **no write**.
7. If `confirm` true and digest matches → resolve `project_dir`, `load_config`,
   `DeployAdapterConfig::from_env()`, then
   `run_with_operation(db_path, "admin.deploy", id, || apply_deploy(project_dir,
   &manifest, &config, &DeployRequest { project_id, target, confirm:true,
   dry_run:false }, &adapters))`. Return 202 with the path-free `DeployReport`
   (stages, observation, `healthy`, state note) and the operation id.

State transitions: none in Forge's own store except the existing `operations`
journal row and `.forge/deploy/.../state.json` written by `apply_deploy`.
Idempotency: the operation journal keys the run; a re-apply is a new confirmed
operation (deploy is not auto-idempotent — the operator re-confirms).
Concurrency: single admin; the existing per-operation journaling applies.

## 5. Contract and compatibility

- New route consts added to `IMPLEMENTED_WEB_ROUTES`:
  `WEB_ROUTE_ADMIN_DEPLOY_PLAN = "GET /v1/admin/projects/{id}/deploy/plan"`,
  `WEB_ROUTE_ADMIN_DEPLOY = "POST /v1/admin/projects/{id}/deploy"`.
- Request body (`POST`): `{ "target": string?, "confirm": bool, "plan_digest":
  string? }`. Only these keys are read; anything else is ignored, never treated
  as a path/argv/shell.
- Catalog `execution` block for `deploy.apply`: `{ route: "POST
  /v1/admin/projects/{id}/deploy", method: "POST", risk: "remote_write",
  confirm_required: true, digest_bound: true, parameters: [ { name: "target",
  kind: "string", required: false } ] }`. `deploy.plan` is a `web` read row (no
  `execution` block) pointing at the plan route.
- Response shapes are additive; the bearer deploy route and every existing
  response are unchanged. No schema/migration.
- Validation ownership: id validation and JSON/session gates in `admin.rs`;
  deploy-domain validation (target existence, SSH refusal, artifact capture)
  stays in `deploy::engine` and surfaces as its typed errors.

## 6. Failure and boundary policy

| Case | Behavior |
|---|---|
| No / invalid session cookie | 401, no Core call |
| Non-JSON POST | 415 |
| Path-bearing or empty project id | 400 `admin-invalid-project-id`, no echo |
| Unknown / unmanaged project id | 404 `admin-project-unmanaged` |
| `confirm` absent/false | 200 preview + digest, **no write** |
| Digest mismatch | 409 `admin-digest-mismatch`, fresh digest, **no write** |
| Target not in `DeployConfig` | `prepare_deploy`/`apply_deploy` typed error → 400/409, no fake success |
| SSH / unsupported target kind | existing `prepare_deploy` `DeployInvalid` refusal, surfaced honestly |
| `FORGE_DEPLOYER_BIN` unset / binary missing | adapter invocation fails → typed failed report, journaled, returned as failure (never success) |
| Adapter returns non-zero / unhealthy | partial/failed `DeployReport` with stage outcomes + observation, `healthy:false`, journaled |
| Any absolute path / adapter binary in a response | scrubbed before return (path-redaction as in `delivery.rs`) |

No case is silently swallowed; a failed remote write is reported as failed,
consistent with "partial external outcomes remain partial."

## 7. Verification oracle

- **New file `tests/forge_web_project_deployment_contract.rs`** (mirrors
  `forge_web_project_actions_contract.rs`), asserting against a temp registry
  seeded with one managed project:
  1. `GET …/deploy/plan` and `POST …/deploy` without a session → 401; POST with
     non-JSON → 415; both run no Core operation.
  2. Path-bearing id → 400 with no path echo; unmanaged id → 404.
  3. `POST …/deploy` without `confirm` → 200 with a 64-hex `plan_digest` and a
     path-free preview; assert the on-disk `.forge/deploy` tree and registry
     `operations` table are unchanged (no write).
  4. `POST …/deploy` with `confirm:true` and a wrong digest → 409
     `admin-digest-mismatch`, fresh digest, no write.
  5. Confirmed run with a matching digest **delegates to `apply_deploy`**: point
     `FORGE_DEPLOYER_BIN` at a fixture stub script (checked into
     `tests/fixtures/`) that reads the adapter JSON on stdin and writes a valid
     `AdapterResponse` to stdout; assert 202, a `deploy` `operations` row is
     journaled, and the response carries the stub's observation with no absolute
     path.
  6. Adapter-failure case: a stub that exits non-zero → the route returns a
     typed failure and journals it, never a success.
  7. Catalog: `deploy.apply` is `web` with an `execution` block naming the admin
     route; `deploy.plan` is `web` read; both routes are in
     `IMPLEMENTED_WEB_ROUTES`.
- **In-source catalog tests** (`src/api/command_catalog.rs`): update the pinned
  `executable_ids` from the 5 to include `deploy.apply` (6 total); the
  `catalog_integrity_is_clean`, `catalog_covers_every_clap_path` (count stays
  227 — dispositions change, not the command set) and
  `web_rows_only_point_at_implemented_routes` tests must pass.
- **Existing contract files** must stay green:
  `forge_web_command_catalog_contract`, `forge_web_command_execution_contract`,
  `forge_web_project_actions_contract`.
- **Frontend:** a headless-Chromium drive against a throwaway registry opens a
  managed project, renders the deploy control from the catalog, previews (digest,
  no write) and — with the stub deployer — applies, with zero console/network
  errors and no absolute path in the DOM. Recorded as evidence, not assumed.
- A task box is checked only with the command output for its assertion.

## 8. Decision ledger

- **Resolved:** deploy is exposed on the admin surface reusing `deploy::engine`
  unchanged (extend, not reimplement); plan is a read route and apply is a
  confirm→digest `web_exec`; the `target` field is optional and server-resolves
  the default.
- **Resolved:** the bearer `/v1` deploy route is left untouched; the admin route
  is a sibling consumer, not a replacement.
- **Deferred to follow-on packages:** release (git-network + subprocess + shell
  check path) and publish/provider delivery (live credentials + health-gated
  promotion) — each its own package and oracle.
- **Deferred (out of scope):** a browser candidate list for `target` derived
  from the project's `DeployConfig`; the base change accepts a typed string.
- **Blockers:** none. The engine, gate pattern, catalog builder and route
  allowlist all exist; no unresolved material decision.
