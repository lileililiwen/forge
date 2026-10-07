# Proposal: Execute the project deploy lifecycle from the browser

## Why

The portal today executes only five project commands (`feature add/remove/upgrade`,
`spec generate/apply`). A headless-Chromium drive against a throwaway registry
confirmed that `forge deploy` — the operation the product brief calls the core of
"manage its lifecycle" (`requirement.md` §11) and the v0.5 "true project lifecycle
platform" (§43) — is catalogued as `project_capability_required` with no
`execution` block, so the browser lists it and says "run it in a terminal." The
user reports they cannot manage the project lifecycle from the portal.

Deploy is the first lifecycle operation that can be lifted without a new risk
surface: Forge already has an in-process, server-side deploy path.
`handle_apply_deployment` (`src/api/mod.rs:3080`) resolves the project directory
from the registry id (never a browser-supplied path), requires `confirm`, and
calls the same `deploy::engine::prepare_deploy` / `apply_deploy` the CLI uses.
That is exactly the "Core only" shape the browser already executes for
feature/spec. This change exposes deploy on the session-gated admin surface
behind the existing preview→confirm→digest gate, so a signed-in operator can plan
and apply a deploy from the portal while credentials, paths and the adapter
binary stay server-side.

## What Changes

- Add two session-gated admin routes for one managed project id:
  - `GET  /v1/admin/projects/{id}/deploy/plan` — read-only; runs
    `prepare_deploy`, returns the bounded plan, writes nothing.
  - `POST /v1/admin/projects/{id}/deploy` — the confirm-gated apply; a request
    without `confirm` returns a `plan_digest` and writes nothing; a request with
    `confirm` true and a matching digest delegates to `apply_deploy`.
- Recatalogue `deploy plan` as `web` (read) and `deploy apply` as `web` with a
  structured `execution` block (the sixth executable row), and add both routes to
  `IMPLEMENTED_WEB_ROUTES`.
- Render the deploy control in the workbench from the catalog `execution` block
  (typed `target` field only — no free-text command, path or argv).
- Preserve every existing invariant: no shell/argv from the browser, no
  browser-supplied path, provider credentials and the `forge-deployer` binary
  resolved only from server env, partial/failed adapter outcomes reported
  honestly and journaled, never as success.

## Package Boundary and Split Assessment

The requested tier ("Deploy/release/publish") contains three independently
verifiable outcomes with different owners, side-effect classes and acceptance
oracles, so it is split into a dependency-ordered package map. Only the first is
authored to implementation-ready detail here; the others are named with their
dependency edge and are not redefined or partially implemented.

| Package | Single outcome | Owner / language | Boundary / contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-web-project-deployment` (**this**) | A signed-in operator plans and applies a **deploy** for one managed project from the browser through the confirm→digest gate | Forge `src/api` + `src/deploy` / Rust | `GET …/deploy/plan`, `POST …/deploy`; `deploy::engine` reused unchanged; catalog `execution` block | `forge-web-project-actions`, `forge-web-command-execution` | `forge_web_project_deployment_contract` + workbench catalog row assertions |
| `forge-web-project-release` (follow-on) | Prepare and apply a **release** (semver/changelog/evidence plan; commit/tag/push/mirror/package/container/notes stages) | Forge `src/api` + `src/release` + `src/gitops` + `src/distribution` / Rust | New admin routes over `release::engine`; git-network + subprocess stages | this package (reuses the admin confirm→digest deploy shape) | release contract test with a git fixture + stub stage adapters |
| `forge-web-project-publish` (follow-on) | Drive **provider publish** (Jenkins/Mac) and **OpenPanel delivery** (preflight→stage→promote) with health-gated promotion | Forge `src/api` + `src/publish` + `src/delivery` / Rust | New admin routes over `publish::providers` / `delivery::handlers`; provider config + `FORGE_*` env prerequisite | `forge-web-project-deployment` (shared prerequisite-config pattern) | publish/delivery contract test with a `RecordingTransport` provider stub |

**Why deploy is the smallest independently verifiable unit:** it is the only one
of the three whose Core operation is already reachable through an existing,
tested server-side route (`handle_apply_deployment`) that resolves everything
from the registry id, so it introduces no new subprocess, git-network or
provider-credential surface beyond what the CLI already runs. Release adds
git-network push and a shell-driven check path; publish adds live provider
credentials and health-gated promotion. Each of those needs its own oracle and
its own blast-radius review, so they stay separate packages.

## Sibling and Shared Architecture Reconnaissance

This repository *is* the manager project; there is no sibling `common`/`manager`
to adopt from, and no shared `/lib` extraction is in scope. The reconnaissance is
therefore intra-repo: locate the existing owner of "browser-executable Core
command" and extend it rather than build a parallel mechanism.

| Candidate | Evidence path / symbol | Reusable code / contract | Compatibility gap | Owner / release boundary | Decision |
|---|---|---|---|---|---|
| Existing deploy server path | `src/api/mod.rs:3080` `handle_apply_deployment`; `src/deploy/engine.rs` `prepare_deploy`/`apply_deploy` | Full plan/apply semantics, `DeployRequest`, `DeployConfig`, `DeployAdapterConfig::from_env`, `run_with_operation` journaling | It is a **bearer** route (`POST /v1/projects/{id}/deployments`), not on the cookie-session admin surface, and has no preview→confirm→digest gate | `src/deploy` owns the engine; `src/api` owns routes | **extend shared owner** — reuse `deploy::engine` unchanged; add an admin-surface route that mirrors the bearer path |
| Admin authoring gate | `src/api/admin.rs` `Authoring` enum, `authoring_descriptor`, `authoring_digest`, `authoring_write` | The exact preview→confirm→digest, id gate, 415/401/404/409 discipline the browser already uses | Enum is authoring-only; deploy is a different risk (RemoteWrite) and needs the adapter path | `src/api/admin.rs` owns the admin gate | **extend shared owner** — add a deploy kind to the same gate, not a new gate |
| Delivery confirm→digest | `src/api/delivery.rs` `confirmed_digest`, `publish()` prerequisite refusal, `redact_local_paths` | A second, independent confirm→digest implementation + path redaction + "prerequisite unset → typed 409" pattern | It is the portfolio-**share** pipeline, a different subsystem from project deploy | `src/api/delivery.rs` owns share delivery | **keep local / reference only** — copy the discipline (digest, redaction, prerequisite refusal); do not route project deploy through the share pipeline |
| Command catalog execution model | `src/api/command_catalog.rs` `web_exec`, `CommandExecution`/`ExecParameter`, `IMPLEMENTED_WEB_ROUTES`, pinned `executable_ids` test | The self-describing row the frontend renders generically | Only 5 rows are executable today | `src/api/command_catalog.rs` owns the catalog | **extend shared owner** — add the deploy rows through the existing `web_exec` builder |

No shared extraction or sibling edit is authorized; every decision is an
in-repo extension of an existing owner.

## BFS Impact Map

- **Capabilities:** `forge-web-project-deployment` (new); `forge-web-command-catalog`
  (modified: deploy rows gain `web`/`execution`); `forge-web-command-execution`
  (modified: the "deploy SHALL remain behind delivery controls" clause is
  superseded for the admin deploy route).
- **Users / flows:** the single global admin operator, in the workbench, on one
  registered managed project.
- **Contracts / data / persistence:** no schema change. Deploy state persists
  under `.forge/deploy/<id>/<deploy-id>/state.json` and one `deploy` row in the
  existing registry `operations` table, both already written by `apply_deploy`.
  New JSON request/response shapes for the two admin routes (path-free).
- **Integrations / configuration:** reads `FORGE_DEPLOYER_BIN` and
  `DEPLOY_ADAPTER_TIMEOUT` from the server env (unchanged); the browser never
  supplies a binary, path, host or credential.
- **Callers:** `src/api/mod.rs` router (new arms + group/CORS matching),
  `src/api/admin.rs` (new gate kind), `src/api/command_catalog.rs` (rows +
  allowlist + `executable_ids` test), `frontend/app.js` (renders the new
  `execution` row generically — no bespoke code expected).
- **Failure / boundary behavior:** unmanaged id → 404; path-bearing id → 400
  (no echo); non-JSON → 415; missing session → 401; no confirm → preview +
  digest, no write; digest mismatch → 409 with a fresh digest; adapter binary
  absent or adapter failure → typed failed/partial report, journaled, never a
  fake success; SSH/unsupported target → the existing `prepare_deploy` refusal.
- **Tests:** new `tests/forge_web_project_deployment_contract.rs`; update the
  catalog in-source `executable_ids` pin (5 → 6) and route allowlist; the
  command-execution and command-catalog contract files must stay green.
- **Dependencies / compatibility:** depends on the archived
  `forge-web-project-actions` and `forge-web-command-execution` shapes; no
  breaking change to the bearer `/v1` deploy route.
- **Privacy / security:** credentials and absolute paths stay server-side;
  responses are path-redacted; the deploy adapter is invoked with a fixed argv
  array exactly as the CLI does, never from browser text.

## Capabilities

- `forge-web-project-deployment` — new: browser planning and confirmed,
  digest-bound application of a project deploy through the session-gated admin
  surface, reusing the in-process deploy engine.
- `forge-web-command-catalog` — modified: deploy rows report their real admin
  routes; `deploy apply` carries an `execution` block.
- `forge-web-command-execution` — modified: deploy is now an admin-surface
  executable command under the same confirm→digest discipline as authoring.

## Non-goals

- No browser execution of `release`, `publish`, provider/OpenPanel delivery,
  git commit/push/mirror, `docs translate`, agent PTY, or build/test (`gate`,
  `test`) — each is a separate follow-on package or stays CLI-only.
- No new persistence schema, no new provider-credential handling, no
  browser-supplied path/binary/host, and no generic shell or argv execution.
- No change to the bearer `/v1/projects/{id}/deployments` route semantics.
- No auto-configuration of `FORGE_DEPLOYER_BIN`; if it is unset the browser
  reports the typed prerequisite/failure honestly, matching the terminal.
