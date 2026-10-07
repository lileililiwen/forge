# Proposal: Drive staged project delivery from the browser

## Why

The browser can plan and run provider publish, but it cannot drive staged delivery. The delivery commands remain `provider_required` or `not_yet_web`, while the project-scoped bearer routes cannot be used by the global workbench session. Operators therefore leave the portal to run preflight, stage, production promotion and Hermora onboarding in a terminal.

## What Changes

- Add session-gated project delivery routes under `/v1/admin/projects/{id}/delivery`.
- Expose read-only delivery status and confirm/digest-bound preflight, stage, promote and Hermora-retry operations.
- Reuse `delivery::handlers` unchanged; resolve project, provider configuration and revision server-side.
- Recatalogue all five delivery commands as `web`, with executable blocks for the four mutations.
- Add a workbench delivery card that shows phase, revision, latest evidence and the next required confirmation; reuse generic action controls for mutations.
- Preserve typed refusals, health gating, idempotency, secret references and path redaction.

## Package Boundary and Split Assessment

This is one coherent staged-delivery unit. Status, preflight, stage, promotion and optional Hermora enrollment share the same journal, revision binding and confirmation state; splitting them would create unverifiable intermediate browser states. It depends on the archived publish package and reuses its provider fixture pattern. No additional split is proposed.

| Package | Single outcome | Owner / language | Boundary / contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-web-project-delivery` (**this**) | A signed-in operator completes evidence-gated preflight → stage → production promotion, with optional Hermora enrollment, from the workbench | Forge `src/api` + `src/delivery` / Rust, `frontend/` | Five new admin routes; `delivery::handlers` reused unchanged | `forge-web-project-publish` | `tests/forge_web_project_delivery_contract.rs` plus a Playwright workbench drive |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path / symbol | Reusable code / contract | Compatibility gap | Owner / release boundary | Decision |
|---|---|---|---|---|---|
| Delivery orchestration | `src/delivery/handlers.rs` `run_status`, `run_preflight`, `run_stage`, `run_promote`, `run_hermora_retry` | State machine, idempotency, health gating, journal projection | Functions are reachable only through CLI and project-bearer routes | `src/delivery` owns orchestration; `src/api` owns routes | **extend shared owner** — add admin facades without changing orchestration; correct only the Hermora detail decoder that otherwise drops a valid connected report |
| Delivery projection decoding | `src/delivery/projection.rs` `populate_hermora` | Connected detail parsing | `BTreeMap<String, String>` cannot parse `"reason": null`, discarding `site_id` | `src/delivery` owns projection | **narrow in-owner correction** — decode optional strings as JSON values; no state-machine change |
| Provider boundary | `src/delivery/invoke.rs`, `src/publish/providers.rs` | Fixed contract, fixed argv, timeout, secret scan | Browser must not choose provider/argv/path | `src/publish` owns provider execution | **reuse unchanged** |
| Hermora boundary | `src/delivery/hermora.rs` | Bounded adapter request/response, credential-shape refusal | Browser must supply only URL and secret reference | `src/delivery` owns adapter semantics | **reuse unchanged** |
| Admin confirm/digest gate | `src/api/admin.rs` `guarded`, `is_json`, `authoring_digest`, `deploy_id_gate`, `scrub_json` | Session, JSON, digest and redaction discipline | Delivery needs revision- and confirmation-bound descriptors | `src/api/admin.rs` owns the gate | **extend shared owner** |
| Catalog execution model | `src/api/command_catalog.rs` `web_at`, `web_exec`, `IMPLEMENTED_WEB_ROUTES` | Runnable typed route declarations | Five delivery rows are not web today | `src/api/command_catalog.rs` owns dispositions | **extend shared owner** |
| Workbench actions | `frontend/app.js` `buildActionControl`, `renderProjectActions` | Generic preview/confirm/run controls | No delivery status/next-action presentation | `frontend/` owns presentation | **extend shared owner** — add one delivery card |

No shared extraction or sibling edit is authorized.

## BFS Impact Map

- **Capabilities:** `forge-web-project-delivery` (new); `forge-web-command-catalog` and `forge-web-command-execution` (modified).
- **Users / flows:** the authenticated global admin opens a managed project, reads its delivery phase, then previews/confirms each staged mutation.
- **Contracts / data / persistence:** no schema change. Existing `delivery.preflight`, `delivery.stage`, `delivery.promote` and `delivery.hermora` journal rows are written by Core exactly as the CLI writes them.
- **Integrations / configuration:** server-side `openpanel` provider configuration, committed revision, stubbed Hermora adapter in tests, and server environment only.
- **Callers:** `src/api/mod.rs` router and exhaustive admin lists; `src/api/admin.rs` facades; `src/api/command_catalog.rs` rows; `tests/forge_web_command_catalog_contract.rs`; `frontend/app.js` and `frontend/index.html`; new contract and browser tests.
- **Failure / boundary behavior:** anonymous, non-JSON, hostile/unmanaged, missing provider, failed provider, unhealthy stage, missing promotion and credential-shaped values are typed and honest.
- **Tests:** new API contract plus a real Chromium workbench flow; existing delivery, publish, catalog and portal suites remain green.
- **Privacy / security:** no browser path, binary, argv, host, credential value or shell text; absolute paths and secrets are scrubbed.

## Capabilities

- `forge-web-project-delivery`: staged project delivery in the workbench.
- `forge-web-command-catalog`: five delivery commands become truthfully `web`.
- `forge-web-command-execution`: four delivery mutations join confirm/digest execution.

## Non-goals

- No browser selection of provider, project path, adapter binary, argv, remote, SSH target, revision source or secret value.
- No change to CLI delivery dispatch, bearer delivery routes, provider/Hermora contracts, journal schema or `API_CONTRACT_VERSION`.
- No automatic promotion, blind retry, re-publication from Hermora retry, live provider probing on page load, or sibling-runtime calls.
