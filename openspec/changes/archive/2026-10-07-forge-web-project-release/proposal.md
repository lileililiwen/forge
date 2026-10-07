# Proposal: Execute the project release lifecycle from the browser

## Why

The archived `forge-web-project-deployment` change lifted `forge deploy` onto the
session-gated admin surface behind the preview→confirm→digest gate, and closed
with release explicitly deferred as a follow-on because it adds git-network push
and subprocess adapter stages. The product brief calls the release/publish tier
part of "manage its lifecycle" (`requirement.md` §11) and the v0.5 "true project
lifecycle platform" (§43); today `release prepare` and `release apply` are
catalogued as `not_yet_web`/`provider_required` with no `execution` block, so the
browser lists them and says "run it in a terminal."

Forge already owns the whole release semantics in-process:
`release::engine::prepare_release` / `apply_release`, `ReleaseConfig`,
`ReleaseAdapterConfig::from_env`, and the `release` operation journal row the CLI
writes. This change exposes that same engine on the cookie-session admin surface
under the exact preview→confirm→digest discipline the feature/spec/deploy routes
use, so a signed-in operator can plan and apply a release from the portal while
credentials, absolute paths, git remotes and the adapter binaries stay
server-side.

## What Changes

- Add two session-gated admin routes for one managed project id:
  - `GET  /v1/admin/projects/{id}/release/plan?version=<semver>` — read-only;
    runs `prepare_release`, returns a bounded path-free plan and the
    `plan_digest`, writes nothing.
  - `POST /v1/admin/projects/{id}/release` — the confirm-gated apply; a request
    without `confirm` returns a plan preview and `plan_digest` and writes
    nothing; a request with `confirm` true and a matching digest delegates to
    `apply_release` and journals a `release` operation.
- Require a typed semver `version` parameter on both routes (query for the plan,
  JSON body for the apply), validated with `Semver::parse` and normalized to its
  canonical label before the digest is bound. No `stages`, path, argv, host,
  remote or credential is ever accepted from the browser.
- Recatalogue `release prepare` as `web` (read) and `release apply` as `web_exec`
  with a structured `execution` block (the seventh executable row), and add both
  routes to `IMPLEMENTED_WEB_ROUTES`.
- Render the release control in the workbench from the catalog `execution` block
  (typed `version` field only — no free-text command, path or argv); no bespoke
  frontend code is expected.
- Preserve every existing invariant: the release state file path, the working
  tree and the adapter binaries stay server-side; partial/failed stage outcomes
  are reported honestly and journaled, never as success.

## Package Boundary and Split Assessment

The requested tier ("Deploy/release/publish") is a dependency-ordered package
map authored by `forge-web-project-deployment`; this change is the second
package. It is independently verifiable because it has its own engine
(`release::engine`), its own persistence (`.forge/release/.../state.json` plus a
`release` journal row) and its own oracle (a hermetic git fixture + stub stage
adapters). It depends on the deploy package only for the now-established
admin-surface confirm→digest shape, which it reuses rather than redefines.

| Package | Single outcome | Owner / language | Boundary / contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-web-project-deployment` (archived) | Plan and apply a **deploy** from the browser | Forge `src/api` + `src/deploy` / Rust | `GET …/deploy/plan`, `POST …/deploy` | `forge-web-project-actions`, `forge-web-command-execution` | deployment contract test |
| `forge-web-project-release` (**this**) | Plan and apply a **release** for one managed project from the browser through the confirm→digest gate | Forge `src/api` + `src/release` / Rust | `GET …/release/plan`, `POST …/release`; `release::engine` reused unchanged; catalog `execution` block | `forge-web-project-deployment` (reuses the admin confirm→digest shape) | `forge_web_project_release_contract` + catalog row assertions |
| `forge-web-project-publish` (follow-on) | Drive **provider publish** (Jenkins/Mac) and **OpenPanel delivery** with health-gated promotion | Forge `src/api` + `src/publish` + `src/delivery` / Rust | New admin routes over `publish::providers` / `delivery::handlers` | `forge-web-project-deployment` | publish/delivery contract test with a `RecordingTransport` provider stub |

**Why release is the next independently verifiable unit:** its Core engine,
persistence, adapter protocol and redaction already exist and are tested; the
only new surface is the admin route plus the catalog disposition. Publish still
adds live provider credentials and health-gated promotion, so it stays a separate
package.

## Sibling and Shared Architecture Reconnaissance

This repository *is* the manager project; there is no sibling `common`/`manager`
to adopt from, and no shared `/lib` extraction is in scope. The reconnaissance
is intra-repo: the deploy package already established "browser-executable Core
lifecycle command" as an extension of `src/api/admin.rs`; release joins that
owner rather than opening a parallel mechanism.

| Candidate | Evidence path / symbol | Reusable code / contract | Compatibility gap | Owner / release boundary | Decision |
|---|---|---|---|---|---|
| Release engine | `src/release/engine.rs` `prepare_release`/`apply_release`; `src/release/mod.rs` `ReleaseConfig`, `release_config_from_manifest`, `Semver`, `ReleaseAdapterConfig::from_env` | Full plan/apply semantics, stage outcomes, adapter protocol, redaction | Invoked only from the CLI (`src/main.rs`), which resolves a path target directly | `src/release` owns the engine; `src/api` owns routes | **extend shared owner** — reuse `release::engine` unchanged; add an admin route that resolves the project dir from the registry id |
| Admin confirm→digest gate | `src/api/admin.rs` `deploy_write`, `deploy_id_gate`, `authoring_digest`, `run_plan_view`/`plan_view`/`report_view`, `scrub_json`/`redact_local_paths` | The exact preview→confirm→digest, 415/401/404/409 discipline and path scrubbing | Gate is deploy/authoring-specific; release has a typed `version` field and a different report | `src/api/admin.rs` owns the gate | **extend shared owner** — add a release kind to the same gate |
| Command catalog execution model | `src/api/command_catalog.rs` `web_at`, `web_exec`, `IMPLEMENTED_WEB_ROUTES`, pinned tests | The self-describing row the frontend renders generically | `release prepare`/`apply` are not-yet-web today | `src/api/command_catalog.rs` owns the catalog | **extend shared owner** — flip the two rows through the existing builders |

No shared extraction or sibling edit is authorized; every decision is an in-repo
extension of an existing owner.

## BFS Impact Map

- **Capabilities:** `forge-web-project-release` (new); `forge-web-command-catalog`
  (modified: release rows gain `web`/`execution`); `forge-web-command-execution`
  (modified: the release commands join the admin confirm→digest discipline).
- **Users / flows:** the single global admin operator, in the workbench, on one
  registered managed project.
- **Contracts / data / persistence:** no schema change. Release state persists
  under `.forge/release/<project-id>/<release-id>/state.json` and one `release`
  row in the existing registry `operations` table, both already written by
  `apply_release`. New JSON request/response shapes for the two admin routes
  (path-free).
- **Integrations / configuration:** reads `FORGE_PACKAGE_BIN`/`FORGE_CONTAINER_BIN`/
  `FORGE_NOTES_BIN` and the git `origin` remote server-side (unchanged); the
  browser never supplies a binary, path, host, remote or credential.
- **Callers:** `src/api/mod.rs` router (new arms + group/CORS matching, route
  variant, permission/dispatch/authorize lists), `src/api/admin.rs` (new route
  consts, id/version gate, plan/report views), `src/api/command_catalog.rs`
  (rows + allowlist + pinned tests), `tests/forge_web_command_catalog_contract.rs`
  (allowlist + `web_ids`), `frontend/app.js` (renders the new `execution` row
  generically — no bespoke code expected).
- **Failure / boundary behavior:** unmanaged id → 404; path-bearing id → 400
  (no echo); non-JSON → 415; missing session → 401; missing/non-semver `version`
  → 400 (no echo of arbitrary text); no confirm → preview + digest, no write;
  digest mismatch → 409 with a fresh digest; git/check/readiness/stage failure →
  typed failed/partial report with real stage state, journaled, never a fake
  success; identity conflict on re-apply → typed error.
- **Tests:** new `tests/forge_web_project_release_contract.rs`; update the
  catalog in-source pinned `executable_ids` (6 → 7) and the route allowlist, and
  the `forge_web_command_catalog_contract` allowlist + `web_ids` set; the
  command-execution and deployment contract files must stay green.
- **Dependencies / compatibility:** depends on the archived
  `forge-web-project-deployment` shape; no breaking change to the CLI
  `forge release` dispatch or the release engine contract.
- **Privacy / security:** credentials, git remotes and absolute paths stay
  server-side; responses are path-redacted; adapter binaries are invoked with
  fixed argv arrays exactly as the CLI does, never from browser text.

## Capabilities

- `forge-web-project-release` — new: browser planning and confirmed,
  digest-bound application of a project release through the session-gated admin
  surface, reusing the in-process release engine.
- `forge-web-command-catalog` — modified: release rows report their real admin
  routes; `release apply` carries an `execution` block.
- `forge-web-command-execution` — modified: release is now an admin-surface
  executable command under the same confirm→digest discipline as authoring and
  deploy.

## Non-goals

- No browser execution of `publish`, provider/OpenPanel delivery, `release list`/
  `release inspect`, `docs translate`, agent PTY, or build/test (`gate`,
  `test`) — each is a separate follow-on package or stays CLI-only.
- No browser-supplied stage list, path, argv, host, remote or credential; no
  generic shell or argv execution.
- No new persistence schema, no new provider-credential handling, and no change
  to the CLI `forge release` dispatch or the release engine contract.
- No auto-configuration of `FORGE_PACKAGE_BIN`/`FORGE_CONTAINER_BIN`/
  `FORGE_NOTES_BIN` or the git `origin` remote; when they are unset or fail the
  browser reports the typed prerequisite/failure honestly, matching the terminal.
