# Design: Browser-executable project release

## 1. Implementation boundary

- **Repository / project:** this Forge repository (`/home/paul/code/forge`), the
  `forge` crate. No sibling project is touched.
- **Modules changed:**
  - `src/api/mod.rs` — add the `AdminProjectReleasePlan`/`AdminProjectRelease`
    route variants, the two router arms, and the group/CORS, permission,
    dispatch and authorize list entries; export the two route consts through
    `admin`.
  - `src/api/admin.rs` — add the two route consts, a release id + version gate,
    path-free plan/report views and scrub helpers, and delegate to
    `release::engine`.
  - `src/api/command_catalog.rs` — flip `release.prepare`/`release.apply` rows to
    `web`/`web_exec` via the existing builders, extend `IMPLEMENTED_WEB_ROUTES`,
    update the pinned `web` and `executable_ids` tests.
  - `tests/forge_web_command_catalog_contract.rs` — add the two routes to the
    web allowlist and the `web_ids` set.
  - `frontend/app.js` — expected to need **no** change: it already renders any
    `execution` row generically from its typed `parameters`.
- **Modules reused unchanged:** `src/release/mod.rs` (`ReleaseRequest`,
  `ReleaseConfig`, `Semver`, `release_config_from_manifest`,
  `ReleaseAdapterConfig::from_env`, `load_changelog`) and
  `src/release/engine.rs` (`prepare_release`, `apply_release`,
  `PlanReport`/`ReleaseReport`), the registry `operations` journaling used by
  `run_with_operation`.
- **Must NOT change:** any Core release semantics, `CONTRACT_VERSION` of the
  catalog or API, the CLI `forge release` dispatch, the portfolio-share delivery
  pipeline, or the deploy/authoring routes.

## 2. Language and runtime

- Rust 2021, toolchain floor `rustc 1.87` (per `Cargo.toml` `rust-version`).
- Build: `cargo build`. Test: `cargo test` (and `cargo test --bin forge` for the
  in-source catalog tests). Format: `cargo fmt`.
- Frontend: plain ES modules under `frontend/`, served by `forge web serve`; no
  bundler, no new dependency.
- Target platform: Linux loopback server; the release engine shells out to `git`
  and the configured adapter binaries exactly as the CLI already does.

## 3. Ownership and shared code

- `release::engine` owns release semantics; the admin route is a thin consumer,
  exactly as `cmd_release_apply` is. The new code must call the same
  `prepare_release` / `apply_release` and must not reimplement plan/apply.
- `src/api/admin.rs` owns the session-gated confirm→digest discipline shared by
  authoring and deploy; release joins that owner rather than opening a parallel
  gate. The digest is computed over a **path-free** canonical descriptor
  (project id + normalized semver label), mirroring `authoring_digest`.
- No shared `/lib` or sibling extraction; project-local extension of existing
  owners only (per the proposal reconnaissance table).

## 4. Behavioral model

Actor: the single authenticated global admin (opaque `forge_admin_session`
cookie). Scope: exactly one managed project id and one typed semver `version`.

**`GET /v1/admin/projects/{id}/release/plan?version=<semver>`** (read-only):
1. Session gate (`guarded`) → 401 if invalid.
2. `validate_project_id(id)` → 400 `admin-invalid-project-id` (rejects any id
   containing a path); `registry.inspect(id)` → 404 `admin-project-unmanaged`.
3. Resolve `project_dir = PathBuf::from(registry row.path)` **server-side**.
4. Parse `version` (`Semver::parse`) → 400 `admin-invalid-version` when absent
   or non-semver; normalize to `label()`.
5. `release_config_from_manifest` → `ReleaseConfig`; on a missing/unsupported
   `release` section return the scrubbed typed Core error.
6. `prepare_release(project_dir, &manifest, &config, &ReleaseRequest {
   project_id, version, confirm:false, dry_run:true, retry:false,
   stages: config.stages.clone() }, &DriftWatchConfig::from_env())` → `PlanReport`
   (invokes no adapter, writes nothing).
7. Return 200 with a path-free plan view and `plan_digest =
   authoring_digest({"project_id": id, "version": label})`.

**`POST /v1/admin/projects/{id}/release`** (confirm-gated apply):
1. `is_json` → 415 `application/json` required.
2. Session gate → 401.
3. id gate → 400 / 404, then `version` parse → 400, before any digest.
4. Build descriptor `{ project_id, version }`; `plan_digest =
   sha256(canonical descriptor)`.
5. If `confirm` not true → 200 `{ preview: <plan view>, plan_digest,
   confirmation: { requires: ["confirm","plan_digest"] } }`; **no write, no
   adapter call**.
6. If `confirm` true but `plan_digest` mismatched → 409 `admin-digest-mismatch`
   with a fresh preview + digest; **no write**.
7. If `confirm` true and digest matches → resolve `project_dir`,
   `release_config_from_manifest`, `ReleaseAdapterConfig::from_env()`, then
   `run_with_operation(db_path, "release", id, req, |op_id, _registry| {
   apply_release(project_dir, &manifest, &config, &ReleaseRequest { project_id,
   version, confirm:true, dry_run:false, retry:false, stages:
   config.stages.clone() }, &adapters) })`. Return 202 with the path-free
   `ReleaseReport` view (`state_path` omitted) and the operation id.

State transitions: none in Forge's own store except the existing `operations`
journal row and `.forge/release/.../state.json` written by `apply_release`.
Idempotency: the operation journal keys the run; a re-apply is a new confirmed
operation (release is resumable by stage, not auto-idempotent). Concurrency:
single admin; the existing per-operation journaling applies.

## 5. Contract and compatibility

- New route consts added to `IMPLEMENTED_WEB_ROUTES`:
  `ROUTE_ADMIN_RELEASE_PLAN = "GET /v1/admin/projects/{id}/release/plan"`,
  `ROUTE_ADMIN_RELEASE = "POST /v1/admin/projects/{id}/release"`.
- Request body (`POST`): `{ "version": string, "confirm": bool,
  "plan_digest": string? }`. Only these keys are read; anything else is ignored,
  never treated as a stage list, path/argv/shell, host or credential. The plan
  route reads `version` from the query string.
- Catalog `execution` block for `release.apply`: `{ route: "POST
  /v1/admin/projects/{id}/release", method: "POST", risk: "remote_write",
  confirm_required: true, digest_bound: true, parameters: [ { name: "version",
  kind: "string", required: true } ] }`. `release.prepare` is a `web` read row
  (no `execution` block) pointing at the plan route.
- Response shapes are additive; the CLI release output and every existing
  response are unchanged. No schema/migration.
- Validation ownership: id/semver/JSON/session gates in `admin.rs`; release-domain
  validation (changelog presence, checks, staged-tree state, tag identity
  conflicts) stays in `release::engine` and surfaces as its typed errors.

## 6. Failure and boundary policy

| Case | Behavior |
|---|---|
| No / invalid session cookie | 401, no Core call |
| Non-JSON POST | 415 |
| Path-bearing or empty project id | 400 `admin-invalid-project-id`, no echo |
| Unknown / unmanaged project id | 404 `admin-project-unmanaged` |
| Missing or non-semver `version` | 400 `admin-invalid-version`, no echo |
| `confirm` absent/false | 200 preview + digest, **no write** |
| Digest mismatch | 409 `admin-digest-mismatch`, fresh digest, **no write** |
| No `release` section / changelog absent | typed Core error (`release-invalid`), no side effect |
| Required check failed or stale | typed `release-check-failed`, no tag/commit, journaled |
| Tag already exists at a different commit | `conflict` stage outcome in a non-healthy report, prior state preserved |
| `FORGE_*_BIN` unset / adapter missing / non-zero | `failed` stage outcome in a non-healthy report, journaled, never success |
| `origin` remote missing / push rejected | `failed` stage outcome in a non-healthy report, journaled |
| Any absolute path / adapter binary / remote name in a response | scrubbed before return (project dir + the three adapter bin values) |

No case is silently swallowed; a failed remote/stage outcome is reported as
failed or partial, consistent with "partial external outcomes remain partial."

## 7. Verification oracle

- **New file `tests/forge_web_project_release_contract.rs`** (mirrors
  `forge_web_project_deployment_contract.rs`), asserting against a temp registry
  seeded with one managed git project whose manifest declares
  `release: { versioning: semver }` (no checks → all checks disabled →
  `ready: true`):
  1. `GET …/release/plan` and `POST …/release` without a session → 401; POST with
     non-JSON → 415; both run no Core operation.
  2. Path-bearing id → 400 with no path echo; unmanaged id → 404; missing and
     non-semver `version` → 400.
  3. `GET …/release/plan?version=1.0.0` → 200 with a path-free plan, `ready:true`
     and a 64-hex `plan_digest`; assert `.forge/release` and the registry
     `operations` table are unchanged (no write).
  4. `POST …/release` without `confirm` → 200 with a 64-hex `plan_digest` and a
     path-free preview; no write.
  5. `POST …/release` with `confirm:true` and a wrong digest → 409
     `admin-digest-mismatch`, fresh digest, no write.
  6. Confirmed run with a matching digest **delegates to `apply_release`**: a
     local git working tree with the changelog modified uncommitted and a local
     bare `origin` remote; assert 202, `healthy:true`, a `release`
     `operations` row is journaled, `.forge/release` state is written, and the
     response carries no absolute project path and no adapter binary path.
  7. Honest-failure case: a manifest with `release.notes` and a stub
     `FORGE_NOTES_BIN` that exits non-zero → 202 with `healthy:false` and a
     `failed` `notes` stage, journaled, never a success.
  8. Catalog: `release.apply` is `web` with an `execution` block naming the admin
     route and a required `version` parameter; `release.prepare` is `web` read;
     both routes are in `IMPLEMENTED_WEB_ROUTES`.
- **In-source catalog tests** (`src/api/command_catalog.rs`): update the pinned
  `web` tuple list and `executable_ids` (6 → 7, adding `release.apply`); the
  `catalog_has_no_integrity_problems`,
  `catalog_covers_every_clap_path` (count stays 227 — dispositions change, not
  the command set) and `web_rows_only_point_at_implemented_routes` tests must
  pass.
- **Existing contract files** must stay green:
  `forge_web_command_catalog_contract`, `forge_web_command_execution_contract`,
  `forge_web_project_deployment_contract`, `forge_web_project_actions_contract`,
  `forge_web_project_workbench_contract`, `forge_admin_api_contract`,
  `portal_ui_contract`, `release_contract`, `release_cross_surface`.
- A task box is checked only with the command output for its assertion.

## 8. Decision ledger

- **Resolved:** release is exposed on the admin surface reusing
  `release::engine` unchanged (extend, not reimplement); plan is a read route
  and apply is a confirm→digest `web_exec`; `version` is a required typed semver
  field normalized before the digest.
- **Resolved:** the browser never supplies a stage list — the route uses the
  manifest-derived `config.stages`, so the executable row's only parameter is
  `version`.
- **Resolved:** the response omits `ReleaseReport.state_path` and scrubs the
  project directory plus the three adapter bin values, so no filesystem or
  binary location leaves the route.
- **Deferred to follow-on packages:** publish/provider delivery (live
  credentials + health-gated promotion).
- **Deferred (out of scope):** a browser release list/inspect surface and a
  browser-side stage selector; the base change applies the manifest's stages.
- **Blockers:** none. The engine, gate pattern, catalog builder and route
  allowlist all exist; no unresolved material decision.
