# Design: Browser project creation, import and registration

## 1. Implementation boundary

- **Repository / project:** this Forge repository (`/home/paul/code/forge`),
  the `forge` crate. No sibling project is touched.
- **Modules changed:**
  - `src/api/mod.rs` — three `Route` variants, three router arms, the
    admin-route group in `handle`, the unreachable-dispatch arms, and the
    three exported route consts live in `admin.rs`.
  - `src/api/admin.rs` — the new management gate: a `ProjectManagement`
    enum, `management_descriptor`, `projects_root`, the read-only preview
    builders, and `management_write`.
  - `src/api/command_catalog.rs` — recatalogue `new`, `import` and `register`
    from `cli_only` to `web_exec`; extend `IMPLEMENTED_WEB_ROUTES`; update the
    pinned web list, `executable_ids` and typed-parameter expectations.
  - `frontend/app.js` — generalize `buildActionControl` to a route without
    `{id}` and render a dashboard-level "Create or adopt a project" section
    from the catalog's `execution` rows.
  - `frontend/index.html` — the new section container.
- **Modules reused unchanged:** `src/generate/mod.rs` (`normalize_explicit`,
  `generate`), `src/import/mod.rs` (`inspect_import`, `adopt_import`),
  `src/registry/mod.rs` (`register`), `src/api/mod.rs`
  (`run_with_operation`, `API_SYNTHETIC_PROJECT`), the `operations` journal.
- **Must NOT change:** the CLI dispatch in `src/main.rs`, the MCP
  `mcp_create_project`/`mcp_import_project` path-based tools, the bearer
  `/v1/projects` creation route, the workbench plan/apply routes, any Core
  generate/import/registry semantics, and `API_CONTRACT_VERSION`.

## 2. Language and runtime

- Rust 2021, toolchain floor `rustc 1.87` (per `Cargo.toml` `rust-version`).
- Build: `cargo build`. Test: `cargo test` (and `cargo test --bin forge` for
  the in-source catalog tests). Format: `cargo fmt`.
- Frontend: plain ES modules under `frontend/`, served by `forge web serve`;
  no bundler, no new dependency.
- Target platform: Linux loopback server; the API and web listeners run in the
  same process family as today.

## 3. Ownership and shared code

- The Core functions keep their ownership: `generate::generate`,
  `import::adopt_import`, `Registry::register`. The admin routes are thin
  consumers that resolve the destination server-side and pass typed fields;
  they **must not** reimplement generation, detection or registration.
- `src/api/admin.rs` owns the session-gated confirm→digest discipline; the
  management gate joins the authoring/deploy gate in that file rather than
  opening a parallel mechanism. The digest is the existing
  `authoring_digest` over a **path-free** canonical descriptor
  (action + the browser's typed fields). The absolute destination is never
  part of the descriptor, so the digest binds the reviewed fields, not a
  machine location.
- Path redaction reuses the module's `scrub_json` / `scrub_text` /
  `redact_local_paths` discipline already used by the deploy routes.
- No shared `/lib` or sibling extraction; project-local extension of existing
  owners only (per the proposal reconnaissance table).

## 4. Behavioral model

Actor: the single authenticated global admin (opaque `forge_admin_session`
cookie). Scope: the whole workspace, addressed by a validated project name;
there is no pre-existing `{id}`.

**Root resolution (`projects_root`)**:
1. Read `FORGE_ADMIN_PROJECTS_ROOT`; trim; blank/unset → `Err(409
   admin-prerequisite)` with a safe next step. The value is never echoed.
2. Canonicalize the root; it must exist and be a directory, else the same
   typed `409` with a safe reason.
3. `destination = canonical_root.join(name)` where `name` has already passed
   `validate_project_id` (kebab; no `/`, `.`, `..`, uppercase). Because the
   name is a single segment, the join is a direct child of the root; for
   `register`/`import` the existing directory is additionally canonicalized
   and required to be a descendant of the canonical root.

**`POST /v1/admin/projects/new`** (`forge new`):
1. `is_json` → `415`; `guarded` session gate → `401`.
2. Parse only `project` (required kebab), `profile` (required non-empty),
   `name` (optional display name), `features` (optional string array). A
   missing/invalid field is a typed `400`; the offending value is never
   echoed.
3. Descriptor `{ action: "project-new", project, profile, name?, features? }`
   (sorted/deduped features, mirroring `normalize_explicit`); digest =
   `authoring_digest(descriptor)`.
4. No `confirm` → `200` path-free preview (the resolved profile, the project
   name, the resolved feature closure, the destination **leaf only**) +
   `plan_digest` + a confirmation note; no write.
5. `confirm` + mismatched digest → `409 admin-digest-mismatch` + refreshed
   preview + digest; no write.
6. `confirm` + matching digest → resolve the root, build
   `CreationRequest` via `normalize_explicit(Some(profile), Some(project),
   name, &features, &destination, None)`, then
   `run_with_operation(db, "admin.project.new", API_SYNTHETIC_PROJECT, req,
   |op_id, _| generate(&mut Registry::open(db)?, &request))`. Return `202`
   with a path-free `{ created: { id, profile, name, files }, operation_id,
   project_id, contract }` scrubbed of the root/destination.

**`POST /v1/admin/projects/import`** (`forge import`):
1–2. As above; fields `project` (required), `profile` (optional), `id`
   (optional override).
3. Descriptor `{ action: "project-import", project, profile?, id? }`.
4. No confirm → resolve the root and the existing directory, run the
   read-only `inspect_import(&destination, profile)` and return a path-free
   proposal view (suggested profile, confidence, language value,
   `manifest_exists`, alternatives) + digest; no write.
5. Mismatch → `409` + refreshed view.
6. Match → `run_with_operation(db, "admin.project.import",
   API_SYNTHETIC_PROJECT, req, |op_id, _| adopt_import(&mut
   Registry::open(db)?, &destination, profile, id))` and return `202` with a
   path-free `{ imported: { id, name, profile }, operation_id, project_id,
   contract }`.

**`POST /v1/admin/projects/register`** (`forge register`):
1–2. As above; field `project` (required).
3. Descriptor `{ action: "project-register", project }`.
4. No confirm → resolve the existing directory, load the manifest read-only
   and return a path-free `{ manifest: { id, name, profile }, ready }` +
   digest; no write.
5. Mismatch → `409` + refreshed view.
6. Match → `run_with_operation(db, "admin.project.register",
   API_SYNTHETIC_PROJECT, req, |op_id, _| Registry::open(db)?.register(
   &destination, None))` and return `202` with a path-free
   `{ registered: { id, name, profile }, operation_id, project_id,
   contract }`.

State transitions: none in Forge's own store beyond the existing
`operations` journal row; `new`/`import` also write the project tree and
`register`/`import` write project rows exactly as their CLI counterparts do.
Idempotency: `run_with_operation` keys each run by the request's
`Idempotency-Key`, so a replay returns the journaled operation without
re-running the mutation. Concurrency: single admin; the existing
per-operation journaling applies.

## 5. Contract and compatibility

- New route consts (exported from `admin.rs`, named by the catalog):
  - `ROUTE_ADMIN_PROJECT_NEW = "POST /v1/admin/projects/new"`
  - `ROUTE_ADMIN_PROJECT_IMPORT = "POST /v1/admin/projects/import"`
  - `ROUTE_ADMIN_PROJECT_REGISTER = "POST /v1/admin/projects/register"`
- Request body: only the documented keys are read; anything else is ignored
  and never treated as a path/argv/shell.
- Catalog `execution` blocks:
  - `new`: `POST /v1/admin/projects/new`, risk `local_write`, parameters
    `project:string:required`, `profile:string:required`,
    `name:string:optional`, `features:string_array:optional`.
  - `import`: `POST /v1/admin/projects/import`, risk `local_write`,
    parameters `project:string:required`, `profile:string:optional`,
    `id:string:optional`.
  - `register`: `POST /v1/admin/projects/register`, risk `local_write`,
    parameters `project:string:required`.
- Response shapes are additive; no bearer route, workbench route or existing
  response changes. No schema/migration.
- Validation ownership: session/JSON/name/digest gates in `admin.rs`;
  profile/feature/manifest/import-domain validation stays in
  `generate`/`import`/`registry` and surfaces as their typed errors, scrubbed
  of the server root.

## 6. Failure and boundary policy

| Case | Behavior |
|---|---|
| No / invalid session cookie | `401`, no Core call |
| Non-JSON POST | `415` |
| Missing/blank required field | `400 admin-field-required`, value not echoed |
| Path-bearing / non-kebab `project` (`..`, `/`, `%2e`, uppercase) | `400 admin-invalid-project-name`, input not echoed, no Core call |
| `FORGE_ADMIN_PROJECTS_ROOT` unset/blank/nonexistent | `409 admin-prerequisite`, safe next step, value not echoed |
| Target directory absent (`register`/`import`) | typed Core `PathUnavailable` → mapped `4xx`/`409`, no write |
| Ambiguous import / unknown profile / id collision | typed Core error, journaled `failed`, never success |
| Existing dir escapes the root via symlink | typed `409 admin-prerequisite`/`400`, no write |
| `confirm` absent/false | `200` preview + digest, **no write** |
| Digest mismatch | `409 admin-digest-mismatch`, fresh preview + digest, **no write** |
| Any absolute path / credential in a response | scrubbed before return |

No case is silently swallowed; a failed creation/adoption/registration is
reported as failed and journaled, consistent with "partial external outcomes
remain partial".

## 7. Verification oracle

- **New file `tests/forge_web_project_management_contract.rs`** (mirrors
  `forge_web_project_deployment_contract.rs`; serialized with a `SERIAL`
  mutex because `FORGE_ADMIN_PROJECTS_ROOT` is process-global), driving the
  real `handle()` against a throwaway registry with a temp root:
  1. anonymous POST to each route → `401`; session + non-JSON → `415`; no
     directory, registry row or journal entry written.
  2. path-bearing/non-kebab `project` (`..%2F..%2Fetc`) → `400` typed, the
     input is never echoed, no write.
  3. missing `FORGE_ADMIN_PROJECTS_ROOT` → `409 admin-prerequisite` with no
     path/root value in the body.
  4. preview without `confirm` → `200` with a 64-hex `plan_digest` and a
     path-free view; assert the target subtree and the `operations` table
     are unchanged.
  5. `confirm: true` with a wrong digest → `409 admin-digest-mismatch` with a
     refreshed digest; still no write.
  6. confirmed matching digest for `register` adopts an existing manifest
     directory: `202`, the project appears in the registry, an `operations`
     row with kind `admin.project.register` exists, and neither the temp root
     nor the destination path appears in the response body.
  7. confirmed `import` on a recognizable directory (`--profile` supplied)
     with `id` override creates `forge.yaml` under the root and registers it:
     `202`; response has no absolute path.
  8. confirmed `new` with a profile creates the project tree under the root
     and registers it: `202`; response has no absolute path.
  9. honest failure: confirmed `register` on a directory with no manifest →
     typed error (not `2xx`), an `operations` row is `failed` or the call is
     refused before write, and no success is reported.
  10. catalog: `new`, `import`, `register` are `web` with `execution` blocks
      naming the three routes and their typed parameters.
- **In-source catalog tests** (`src/api/command_catalog.rs`): update the
  pinned `web` list, `executable_ids` and typed-parameter map; keep the count
  at 227; `catalog_integrity_is_clean`,
  `web_rows_only_point_at_implemented_routes` and
  `web_execution_rows_are_well_formed_and_point_at_implemented_routes` pass.
- **Existing contract files** must stay green:
  `forge_web_command_catalog_contract` (strict allowlist + `web_ids` updated),
  `forge_web_command_execution_contract`, `forge_web_project_actions_contract`,
  `forge_web_project_workbench_contract`, and the `--bin forge` catalog tests.
- A task box is checked only with the command output for its assertion.

## 8. Decision ledger

- **Resolved:** creation/registration is the first slice; `upgrade` is already
  delivered by the workbench routes and needs no new route; release and
  publish are follow-on packages.
- **Resolved:** the browser names a location only through a validated
  single-segment `project` name; the server joins it to
  `FORGE_ADMIN_PROJECTS_ROOT`. No browser path, ever.
- **Resolved:** a dedicated, **unset-by-default**
  `FORGE_ADMIN_PROJECTS_ROOT` is used rather than the existing
  `FORGE_WORKSPACE_ROOT` (which defaults to `/home/paul/code`), so no browser
  write can target a path the operator did not explicitly declare; the unset
  case is a typed prerequisite exactly like `FORGE_SHARE_PUBLISH_TARGET`.
- **Resolved:** the three routes use the single-request preview/confirm gate
  (`authoring_write` shape) rather than the deploy GET-plan + POST pair,
  because there is no per-project read route to hang a plan off and the
  no-confirm POST already returns the read-only preview.
- **Resolved:** the digest is computed over the path-free typed descriptor, so
  it binds the reviewed fields and never serializes a machine location.
- **Deferred to `forge-web-project-release`:** prepare/apply release.
- **Deferred to `forge-web-project-publish`:** provider publish / delivery.
- **Deferred (out of scope):** a browser candidate list of directories under
  the root; the base change accepts a typed name.
- **Blockers:** none. The Core functions, gate pattern, catalog builder and
  route allowlist all exist; no unresolved material decision.

## 9. Requirement traceability

| Requirement | Design decision / boundary | Scenarios | Task IDs | Verification oracle |
|---|---|---|---|---|
| Browser creation and adoption of projects from structured fields | server-root join, typed fields, read-only preview | signed-in preview; unset root; hostile name; server-resolved destination | 2.1–2.4 | contract cases 1–5, 9 |
| Confirm and digest binding on project-management mutations | `authoring_digest` over the path-free descriptor | no-confirm preview; mismatch refusal; confirmed run | 2.2, 2.5 | contract cases 4–8 |
| Honest outcomes and server-side path privacy | `scrub_json`/`redact_local_paths`, typed Core errors journaled | adapter-free failure reported honestly; no path leak | 2.5, 3.1–3.3 | contract cases 3, 6–9 |
| Catalog agrees with what the browser can run | `web_exec` rows + `IMPLEMENTED_WEB_ROUTES` | executable web row carries an execution block | 2.6, 3.4 | contract case 10 + in-source catalog tests |
