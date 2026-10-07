# Proposal: Manage projects from the browser

## Why

The operator runs a 70+ project portfolio and publishes heavily to the Mac
server, but reports they still cannot *manage* a project from the portal:

- "i have 70+ project, and i publish a lot to the mac server"
- "i can still not manage project"
- "find the project status"
- "the project list is wrong"

Tier 1 (`forge-web-publish-fleet`, archived) made the fleet show the projects
the local publish journal knows, including observed publish-only rows. The
list is now truthful, but every project-mutating command that would let the
operator create or adopt a project is still catalogued as `cli_only`, so the
portal can list a project it cannot bring under management. This change lifts
the creation/registration surface onto the same session-gated,
exact-origin, JSON-only admin boundary the other `/v1/admin` mutations use.

The commands this tier targets are the project-mutating ones:

- `forge new` (create a project)
- `forge import` (import an existing directory)
- `forge register` (register a directory)
- `forge upgrade` (full project upgrade)
- `forge release` (prepare/apply a release)
- `forge publish` (drive the provider publish / fleet chain)

## What Changes

- Add three session-gated, confirm/digest-gated admin routes for the
  creation/registration surface:
  - `POST /v1/admin/projects/new` (`forge new`)
  - `POST /v1/admin/projects/import` (`forge import`)
  - `POST /v1/admin/projects/register` (`forge register`)
- Each route accepts only **structured typed fields** — a validated
  single-segment kebab `project` name plus that command's own scalar fields.
  It **never** accepts a path, host, binary, argv vector or shell string.
- Resolve the project directory only from a server-side configured root
  (`FORGE_ADMIN_PROJECTS_ROOT`) joined with the validated name. An unset root
  is a typed `409 admin-prerequisite` refusal with a safe next step, exactly
  like the delivery surface's `FORGE_SHARE_PUBLISH_TARGET`.
- Delegate to the same in-process Core functions the CLI runs
  (`generate`, `Registry::register`, `adopt_import` / `inspect_import`); no
  subprocess, no shell, no reimplemented semantics.
- Keep every mutation behind preview → confirm → digest: no `confirm: true`
  and no matching `plan_digest` means no write and no Core mutation; a stale
  digest returns a refreshed preview and a `409`.
- Recatalogue `new`, `import` and `register` from `cli_only` to `web_exec`
  rows with an `execution` block naming the real route and typed parameters,
  add the routes to `IMPLEMENTED_WEB_ROUTES`, and render a generic
  project-management control in `frontend/app.js` from the catalog.
- Keep `forge upgrade`, `forge inspect` and `forge doctor` served by the
  existing workbench routes (already `web`); release and publish stay on
  their existing honest dispositions and become follow-on packages.

## Package Boundary and Split Assessment

The requested tier ("manage project" — creation/registration plus
release/publish) contains outcomes with different owners, side-effect classes
and acceptance oracles. It is split into a dependency-ordered package map.
Only the first slice is authored to implementation-ready detail here; the
follow-ons are named with their dependency edge and are **not** partially
implemented.

| Package | Single outcome | Owner / language | Boundary / contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-web-project-management` (**this**) | A signed-in operator creates (`new`), imports (`import`) or registers (`register`) a project from the browser through the confirm→digest gate, resolving the directory only from a server-side root | Forge `src/api` + `src/generate` + `src/import` + `src/registry` / Rust, `frontend/` | Three new `POST /v1/admin/projects/{new,import,register}` routes; `FORGE_ADMIN_PROJECTS_ROOT` prerequisite; catalog `execution` blocks | `forge-web-project-fleet`, `forge-web-project-deployment` | `tests/forge_web_project_management_contract.rs` |
| `forge-web-project-release` (follow-on) | Prepare and apply a **release** (semver/changelog/evidence plan; tag/package/notes stages) | Forge `src/api` + `src/release` + `src/gitops` + `src/distribution` / Rust | New admin routes over `release::engine`; git-network + subprocess stages | this package (reuses the admin confirm→digest + root-resolution shape) and `forge-web-project-deployment` | release contract test with a git fixture + stub stage adapters |
| `forge-web-project-publish` (follow-on) | Drive **provider publish** (Jenkins/Mac) and **OpenPanel delivery** with health-gated promotion | Forge `src/api` + `src/publish` + `src/delivery` / Rust | New admin routes over `publish::providers` / `delivery::handlers`; provider config + `FORGE_*` env prerequisite | this package (shared prerequisite + gate pattern) and `forge-web-project-deployment` | publish/delivery contract test with a `RecordingTransport` provider stub |

**Why creation/registration is the smallest independently verifiable unit:**
all three of its Core operations already exist as in-process functions the
CLI calls and all three are **local writes with no network, credential,
subprocess or provider surface**; the only genuinely new decision is how a
browser names a location without supplying a path, which the server-side-root
rule answers deterministically and testably. `forge upgrade` is already
delivered: its catalog row is `web` pointing at the workbench
`GET /v1/admin/projects/{id}/plan` / `POST /v1/admin/projects/{id}/apply`
routes, which call `plan_upgrade`/`apply_upgrade` under the same gate, so it
needs no new route and is explicitly out of scope. Release adds git-network
push; publish adds live provider credentials and health-gated promotion. Each
of those needs its own oracle and blast-radius review, so they stay separate
packages.

## Sibling and Shared Architecture Reconnaissance

This repository *is* the manager project; there is no sibling
`common`/`manager` to adopt from and no `/lib` extraction is in scope. The
reconnaissance is intra-repo: locate the existing owner of "browser-executable
Core command" and extend it rather than build a parallel mechanism.

| Candidate | Evidence path / symbol | Reusable code / contract | Compatibility gap | Owner | Decision |
|---|---|---|---|---|---|
| Admin authoring gate | `src/api/admin.rs` `Authoring`, `authoring_descriptor`, `authoring_digest`, `authoring_write`, `guarded`, `is_json`, `redact_local_paths`, `scrub_json` | The exact preview→confirm→digest, 415/401/400/404/409 discipline the browser already uses | The gate is keyed by a managed project `{id}`; creation has no pre-existing id and needs a server-side root | `src/api/admin.rs` | **extend shared owner** — add a management gate in the same module, reuse `authoring_digest` and the scrub helpers |
| Project creation | `src/generate/mod.rs` `normalize_explicit`, `generate`; CLI `cmd_new` | Deterministic in-process generation from typed fields | CLI takes a browser-unsafe destination path | `src/generate` / `src/registry` | **reuse unchanged** — the route resolves `destination` server-side and calls `generate` |
| Import adoption | `src/import/mod.rs` `inspect_import`, `adopt_import`; CLI `cmd_import`; MCP `mcp_import_project` | Read-only proposal + accepted adoption, both in-process | MCP accepts a raw `path`; the admin surface must not | `src/import` | **reuse unchanged** — the route resolves `dir` server-side |
| Registration | `src/registry/mod.rs` `Registry::register`, `check_identity_available`; CLI `cmd_register` | Manifest validation + idempotent registration with journaling | CLI takes a path | `src/registry` | **reuse unchanged** — the route resolves `dir` server-side |
| Server-declared target prerequisite | `src/api/delivery.rs` `PUBLISH_TARGET_ENV` (`FORGE_SHARE_PUBLISH_TARGET`), typed `409` when unset | The "operator configures the server-side location, browser never names it" pattern | None | `src/api/delivery.rs` (reference only) | **copy the discipline**, not the code: a `FORGE_ADMIN_PROJECTS_ROOT` prerequisite in the management gate |
| Existing root resolver | `src/publish/fleet.rs` `default_workspace_root` (`$FORGE_WORKSPACE_ROOT`), `src/governance.rs` `WORKSPACE_ROOT_ENV` | A workspace root concept | It is the *code* workspace, defaulted to `/home/paul/code`; coupling project *creation* to it would write without an explicit operator opt-in | `src/publish/fleet.rs` (reference only) | **do not couple** — introduce a dedicated, unset-by-default `FORGE_ADMIN_PROJECTS_ROOT` so no browser write can target a path the operator did not explicitly declare |
| Command catalog execution model | `src/api/command_catalog.rs` `web_exec`, `CommandExecution`/`ExecParameter`, `IMPLEMENTED_WEB_ROUTES`, pinned `executable_ids`/web-list tests | The self-describing row the frontend renders generically | Only lifecycle rows are executable today | `src/api/command_catalog.rs` | **extend shared owner** — recatalogue the three rows through the existing `web_exec` builder |
| Generic action control | `frontend/app.js` `buildActionControl`, `renderProjectActions` | Catalog-driven preview→confirm→run control | It is workbench-scoped and always substitutes `{id}`, but creation routes have no id | `frontend/app.js` | **extend owner** — generalize the control to a route without `{id}` and add a dashboard-level management section |

No shared extraction or sibling edit is authorized; every decision is an
in-repo extension of an existing owner.

## BFS Impact Map

- **Capabilities:** `forge-web-project-management` (new); `forge-web-command-catalog`
  (modified: `new`/`import`/`register` become `web_exec` with real routes);
  `forge-web-command-execution` (modified: the "catalog agrees" clause now
  names the creation rows).
- **Users / flows:** the single global admin operator, on the dashboard, not
  inside a project.
- **Contracts / data / persistence:** no schema change. New JSON
  request/response shapes for the three routes (path-free). Each confirmed
  mutation writes one `operations` journal row via the existing
  `run_with_operation` boundary and, where the Core function already does so,
  the project's own registry rows and files.
- **Integrations / configuration:** reads `FORGE_ADMIN_PROJECTS_ROOT` from the
  server env (new; no default). No provider, network or credential. The
  browser never supplies a binary, path, host or credential.
- **Callers:** `src/api/mod.rs` router/group/dispatch; `src/api/admin.rs`
  (new management gate); `src/api/command_catalog.rs` (rows + allowlist +
  tests); `frontend/app.js` + `frontend/index.html` (management section);
  the new contract test.
- **Failure / boundary behavior:** unset root → `409 admin-prerequisite`;
  invalid/path-bearing name → `400` with no echo; non-JSON → `415`; missing
  session → `401`; no confirm → path-free preview + digest, no write;
  digest mismatch → `409` + fresh digest, no write; Core failure (unknown
  profile, missing directory, ambiguous import, id collision) → typed error,
  journaled, never a fake success.
- **Tests:** new `tests/forge_web_project_management_contract.rs`; the
  catalog contract (`forge_web_command_catalog_contract`) strict allowlist and
  `web_ids` set must be updated; command-execution and the other `forge_web_*`
  contracts must stay green.
- **Dependencies / compatibility:** depends on the archived
  `forge-web-project-fleet` and `forge-web-project-deployment` shapes; no
  breaking change to any bearer `/v1` route or the workbench.
- **Privacy / security:** the browser supplies a single validated segment,
  never a path; responses are scrubbed with `redact_local_paths`/`scrub_json`
  using the resolved root as a secret; no credential, adapter binary or
  absolute path ever leaves the boundary.

## Capabilities

- `forge-web-project-management` — new: browser creation, import and
  registration of projects under a server-declared root, through the
  session-gated confirm→digest admin gate, reusing the in-process Core
  functions the CLI runs.
- `forge-web-command-catalog` — modified: the `new`, `import` and `register`
  rows report their real admin routes and carry `execution` blocks.
- `forge-web-command-execution` — modified: the catalog's "what the browser
  can run" statement includes the creation/registration commands.

## Non-goals

- No browser execution of `release`, `publish`, provider/OpenPanel delivery,
  git commit/push/mirror, `docs translate`, agent PTY, or build/test — each
  is a follow-on package or stays CLI-only.
- No browser-supplied filesystem path, binary, host, argv or credential; the
  destination is always `FORGE_ADMIN_PROJECTS_ROOT` + a validated name.
- No change to `forge upgrade`, `inspect` or `doctor`, which already run
  through the workbench routes.
- No new persistence schema, no new provider-credential handling, and no
  generic shell, eval or argv route.
- No auto-configuration of `FORGE_ADMIN_PROJECTS_ROOT`; if it is unset the
  browser reports the typed prerequisite honestly, matching a terminal's need
  for an explicit destination.
