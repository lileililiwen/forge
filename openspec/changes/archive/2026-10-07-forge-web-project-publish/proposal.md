# Proposal: Drive a provider publish from the browser

## Why

The archived `forge-web-project-deployment` package lifted `forge deploy` onto
the session-gated admin surface and named two follow-ons: `release` (closed by
`forge-web-project-release`) and this package, provider publish. The product
brief calls the publish tier part of "manage its lifecycle"
(`requirement.md` §11) and the v0.5 "true project lifecycle platform" (§43);
today the bare `forge publish` row is catalogued `provider_required` with the
"run it in a terminal" reason, so the browser lists the command and refuses to
run it.

Forge already owns the whole provider-publish semantics in-process:
`publish::providers::{load_config, select_provider, invoke_provider}` and the
`forge-publish-provider/0.1.0` request/response contract, plus the `publish`
operation row (`Registry::record_publish_phase`) the CLI writes. The bearer
GitHub-push handler (`src/api/mod.rs::handle_github_push`) and the CLI
(`src/main.rs::cmd_publish_provider`) both drive the exact same functions. This
change exposes that same path on the cookie-session admin surface under the
preview→confirm→digest discipline, so a signed-in operator can plan and run a
provider publish for one managed project while the provider executable, the
provider id, the project directory, the revision and any credential stay
server-side.

## What Changes

- Add two session-gated admin routes for one managed project id:
  - `GET  /v1/admin/projects/{id}/publish/plan` — read-only; resolves the
    provider id, the provider configuration and the committed git revision
    server-side, runs no provider, writes nothing and returns a bounded
    path-free plan plus the `plan_digest`.
  - `POST /v1/admin/projects/{id}/publish` — the confirm-gated apply; a request
    without `confirm` returns the plan preview and `plan_digest` and writes
    nothing; a request with `confirm` true and a matching digest delegates to
    `publish::providers::invoke_provider`, journals a `publish` operation with
    the provider-reported state and phase evidence, and returns its typed
    result.
- Resolve the project directory, provider id, provider configuration path and
  revision **only** server-side (registry id, `FORGE_PUBLISH_PROVIDER`,
  `FORGE_PUBLISH_PROVIDER_CONFIG` / the project's `.forge/providers.yaml`, and
  `git rev-parse HEAD`). The browser never supplies a path, binary, argv, host,
  SSH target, credential, revision, provider or free-text command; the apply
  body reads only `confirm` and `plan_digest`.
- An unset `FORGE_PUBLISH_PROVIDER`, a missing/invalid provider configuration,
  a provider that is unknown or disabled, or a project with no resolvable
  40-character revision is a typed `409 admin-prerequisite` that names the
  variable or the configuration, never a credential value or an absolute path.
- Recatalogue the bare `publish` row as a `web_exec` row pointing at the apply
  route (zero browser-supplied parameters), add both routes to
  `IMPLEMENTED_WEB_ROUTES`, and keep `publish.provider.*` /
  `publish.sync|db|prepare|deploy|all|fleet` honestly non-web.
- Render the publish control in the workbench from the catalog `execution`
  block (no typed fields; preview→confirm→run); no bespoke frontend code is
  expected.
- Preserve every existing invariant: the provider executable, the project
  directory and the revision stay server-side; a rejected or failed provider is
  reported with its real status/evidence and journaled, never as success.

## Package Boundary and Split Assessment

The deployment package authored a dependency-ordered package map for the
"Deploy/release/publish" tier; this is its third and final package. The
deployment package originally scoped one follow-on covering **both** provider
publish (Jenkins/Mac) and OpenPanel delivery with health-gated promotion. That
combined unit is too large and too weakly coupled to verify as one change: the
provider path is a single synchronous `invoke_provider` subprocess call over the
`forge-publish-provider/0.1.0` contract with no promotion state, while delivery
adds preflight/stage/promote phases, a health gate and the OpenPanel/Hermora
adapters. Per the split rule, **this change implements the provider publish
only** and names `forge-web-project-delivery` as the delivery follow-on, with
the dependency edge recorded below.

| Package | Single outcome | Owner / language | Boundary / contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-web-project-deployment` (archived) | Plan and apply a **deploy** from the browser | Forge `src/api` + `src/deploy` / Rust | `GET …/deploy/plan`, `POST …/deploy` | `forge-web-project-actions`, `forge-web-command-execution` | deployment contract test |
| `forge-web-project-release` (archived) | Plan and apply a **release** from the browser | Forge `src/api` + `src/release` / Rust | `GET …/release/plan`, `POST …/release` | `forge-web-project-deployment` | release contract test |
| `forge-web-project-publish` (**this**) | Plan and run a **provider publish** for one managed project through the confirm→digest gate | Forge `src/api` + `src/publish` / Rust | `GET …/publish/plan`, `POST …/publish`; `publish::providers` reused unchanged | `forge-web-project-deployment` (reuses the admin confirm→digest shape) | `forge_web_project_publish_contract` + catalog row assertions |
| `forge-web-project-delivery` (follow-on) | Drive **OpenPanel delivery** with health-gated preflight→stage→promote and Hermora retry | Forge `src/api` + `src/delivery` / Rust | New admin routes over `delivery::{handlers, invoke}` and the provider stage/promote verbs | `forge-web-project-publish` (reuses the admin provider+dialog shape) | delivery contract test with a `RecordingTransport` provider stub |

**Why provider publish is the next independently verifiable unit:** its Core
engine, provider contract and `publish` journal write already exist and are
tested (`publish_contract`, `forge_web_publish_fleet_contract`); the only new
surface is the admin route plus the catalog disposition. Delivery still adds a
multi-phase promotion state machine and health gating, so it stays a separate
package.

## Sibling and Shared Architecture Reconnaissance

This repository *is* the manager project; there is no sibling `common`/`manager`
to adopt from, and no shared `/lib` extraction is in scope. The reconnaissance
is intra-repo: the deploy/release packages already established "browser-
executable Core lifecycle command" as an extension of `src/api/admin.rs`; publish
joins that owner rather than opening a parallel mechanism.

| Candidate | Evidence path / symbol | Reusable code / contract | Compatibility gap | Owner / release boundary | Decision |
|---|---|---|---|---|---|
| Provider publish engine | `src/publish/providers.rs` `load_config` / `select_provider` / `invoke_provider` / `PublishProviderRequest` / `PublishProviderResponse` / `PUBLISH_PROVIDER_CONTRACT` | Full provider contract, argv, timeout, secret scan | Invoked only from the CLI and the bearer GitHub-push handler | `src/publish` owns the engine; `src/api` owns routes | **extend shared owner** — reuse `publish::providers` unchanged; add an admin route that resolves the project dir/provider/revision server-side |
| Publish journal | `src/registry/mod.rs` `record_publish_phase` / `PublishPhaseEvidence`; `src/api/fleet.rs` `latest_publishes` (`kind IN ('publish','publish.github')`) | One `publish` row carrying state + revision/build/run/container evidence | CLI-only today | `src/registry` owns the journal; `src/api` writes it | **extend shared owner** — write the same `publish` row `cmd_publish_provider` writes |
| Admin confirm→digest gate | `src/api/admin.rs` `deploy_id_gate`, `authoring_digest`, `guarded`, `is_json`, `scrub_json`/`scrub_text` | Exact preview→confirm→digest, 415/401/404/409 discipline and path scrubbing | Gate is deploy/authoring-specific; publish has a server-resolved provider/revision descriptor | `src/api/admin.rs` owns the gate | **extend shared owner** — add a publish kind to the same gate |
| Command catalog execution model | `src/api/command_catalog.rs` `web_exec`, `IMPLEMENTED_WEB_ROUTES`, pinned tests | The self-describing row the frontend renders generically | Bare `publish` is `provider_required` today | `src/api/command_catalog.rs` owns the catalog | **extend shared owner** — flip the row through the existing builder |

No shared extraction or sibling edit is authorized; every decision is an in-repo
extension of an existing owner.

## BFS Impact Map

- **Capabilities:** `forge-web-project-publish` (new); `forge-web-command-catalog`
  (modified: the bare `publish` row gains `web`/`execution`);
  `forge-web-command-execution` (modified: `publish` joins the admin
  confirm→digest discipline).
- **Users / flows:** the single global admin operator, in the workbench, on one
  registered managed project.
- **Contracts / data / persistence:** no schema change. One `publish` row in the
  existing registry `operations` table with the additive
  `revision`/`build_status`/`run_status`/`container_identity` columns, already
  written by `record_publish_phase`. New path-free JSON request/response shapes
  for the two admin routes.
- **Integrations / configuration:** reads `FORGE_PUBLISH_PROVIDER`,
  `FORGE_PUBLISH_PROVIDER_CONFIG` (or the project `.forge/providers.yaml`), the
  provider executable named there, and the git `HEAD` revision server-side
  (unchanged); the browser never supplies a binary, path, host, remote,
  revision, provider or credential.
- **Callers:** `src/api/mod.rs` router (new arms + group/CORS matching, route
  variants, permission/dispatch/authorize lists), `src/api/admin.rs` (new route
  consts, provider/revision resolution, plan/write views),
  `src/api/command_catalog.rs` (row + allowlist + pinned tests),
  `tests/forge_web_command_catalog_contract.rs` (allowlist + `web_ids`),
  `frontend/app.js` (renders the new `execution` row generically — no bespoke
  code expected).
- **Failure / boundary behavior:** unmanaged id → 404; path-bearing id → 400 (no
  echo); non-JSON → 415; missing session → 401; unset provider / unusable
  configuration / unknown or disabled provider / unresolvable revision → typed
  409 `admin-prerequisite` (no path or secret echo); no confirm → preview +
  digest, no write; digest mismatch → 409 with a fresh digest; provider spawn
  failure, timeout, invalid JSON, secret leak or non-zero exit → typed
  non-success, journaled `failed`, never a fake success.
- **Tests:** new `tests/forge_web_project_publish_contract.rs`; update the
  catalog in-source pinned `web` tuple, `executable_ids` (7 → 8) and route
  allowlist, and the `forge_web_command_catalog_contract` allowlist + `web_ids`
  set; the release/deploy/command-execution contract files must stay green.
- **Dependencies / compatibility:** depends on the archived
  `forge-web-project-deployment` shape; no breaking change to the CLI
  `forge publish` dispatch, the provider contract or the delivery adapters.
- **Privacy / security:** credentials, the provider executable, the provider id
  and every absolute path stay server-side; responses are path/secret-scrubbed;
  the provider is invoked with a fixed argv exactly as the CLI does, never from
  browser text.
- **Split:** provider publish ships here; OpenPanel delivery with health-gated
  promotion is `forge-web-project-delivery`, depending on this change.

## Capabilities

- `forge-web-project-publish` — new: browser planning and confirmed,
  digest-bound execution of a provider publish through the session-gated admin
  surface, reusing the in-process provider contract.
- `forge-web-command-catalog` — modified: the bare `publish` row reports its
  real admin route and carries an `execution` block.
- `forge-web-command-execution` — modified: `publish` is now an admin-surface
  executable command under the same confirm→digest discipline as authoring,
  deploy and release.

## Non-goals

- No browser execution of OpenPanel/health-gated delivery (`delivery
  preflight`/`stage`/`promote`/`hermora-retry`), `publish.provider.*`,
  `publish.sync|db|prepare|deploy|all|fleet`, `release list`/`inspect`,
  `docs translate`, agent PTY, or build/test — each is the delivery follow-on or
  stays CLI-only.
- No browser-supplied provider, revision, path, argv, host, remote, SSH target,
  credential or stage list; no generic shell or argv execution.
- No new persistence schema, no new provider-credential handling, and no change
  to the CLI `forge publish` dispatch or the `forge-publish-provider/0.1.0`
  contract.
- No auto-configuration of `FORGE_PUBLISH_PROVIDER` /
  `FORGE_PUBLISH_PROVIDER_CONFIG`; when they are unset or fail the browser
  reports the typed prerequisite honestly, matching the terminal.
