# Design: Browser-executable staged delivery

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge` crate. No sibling project is touched.
- **Modules changed:**
  - `src/api/mod.rs` — five `Route` variants, five router arms, admin short-circuit, permission, dispatch and unreachable-list entries.
  - `src/api/admin.rs` — five route consts, `DeliveryAction` descriptors, registered-revision resolution, typed preview builders, `delivery_status` and `delivery_write`, Core-error mapping and scrubbing.
  - `src/api/command_catalog.rs` — five delivery rows become `web`; four mutation rows gain `execution`; five routes join `IMPLEMENTED_WEB_ROUTES`; pinned web/executable/parameter assertions updated.
  - `src/delivery/projection.rs` — decode optional Hermora detail fields as JSON values so a valid connected report retains `site_id`; add a connected-detail unit test.
  - `tests/forge_web_command_catalog_contract.rs` — five routes join the web allowlist; five ids join `web_ids`.
  - `frontend/app.js` — add `loadProjectDelivery`, `renderProjectDelivery`, next-action text and refresh after delivery mutations.
  - `frontend/index.html` — add the workbench delivery card and live region.
  - `tests/forge_web_project_delivery_contract.rs` — new API oracle.
  - `tests/browser/delivery-workbench-check.mjs` and `tests/forge_web_project_delivery_browser.rs` — real Chromium flow oracle.
- **Modules reused unchanged:** `src/delivery/{handlers,invoke,state,hermora}.rs`, `src/publish/providers.rs`, `src/registry/mod.rs`, CLI delivery dispatch and bearer delivery handlers.
- **Narrow Core correction:** `src/delivery/projection.rs` decodes optional Hermora detail fields as JSON values instead of requiring every value to be a string. Connected responses carry `"reason": null`; the old decoder discarded the entire detail object, including `site_id`, and therefore never reached `hermora-connected`. No orchestration, journal or adapter behavior changes.
- **Must NOT change:** CLI behavior, bearer routes, provider/Hermora contracts, journal schema, `API_CONTRACT_VERSION`, share-delivery routes, or unrelated catalog rows.

## 2. Language and runtime

- Rust 2021, toolchain floor `rustc 1.87`.
- Build: `cargo build`. Test: focused `cargo test --test ...`, `cargo test --bin forge`, `cargo test --lib api::`.
- Frontend: standalone HTML/CSS/JavaScript served by `forge web serve`; no bundler or dependency.
- Browser oracle: pinned Playwright/Chromium under `tests/browser`; `UNVERIFIED` when unavailable, never a pass.
- Target platform: Linux loopback API and web listeners with hermetic provider/Hermora stubs.

## 3. Ownership and shared code

- Delivery semantics stay in `src/delivery`. Admin facades validate transport concerns only: session, JSON, project id, typed browser fields, preview digest and response projection.
- `deploy_id_gate` remains the managed-project gate: valid kebab id, registered project, server-resolved directory.
- Registered revision comes from `Registry::inspect(id).last_commit`, validated as 40 hexadecimal characters. This is the same revision Core uses.
- Preview digests use `authoring_digest` over path-free descriptors:
  - preflight: project and revision.
  - stage: project, revision and canonical decimal operation id.
  - promote: project, registered revision and supplied revision.
  - Hermora: project, revision, URL and secret reference.
- Responses reuse the existing delivery projection and typed Core errors. Absolute project paths are scrubbed with `scrub_json`; Core already scrubs credentials.
- The catalog remains the single executable-route source. Frontend mutation controls remain generic.

## 4. Behavioral model

Actor: authenticated global admin using the `forge_admin_session` cookie. Scope: one managed project.

### Routes

- `GET /v1/admin/projects/{id}/delivery/status`
  1. `guarded` session gate.
  2. Managed-project gate.
  3. `Registry::open_read_only` plus `handlers::run_status`; no provider, adapter or journal write.
  4. Return 200 with the scrubbed projection, next eligible action and required confirmation fields. No provider call or write.

- `POST /v1/admin/projects/{id}/delivery/preflight`
  1. JSON-only 415 gate.
  2. Session gate.
  3. Resolve revision.
  4. No `confirm`: 200 preview plus digest; no provider or journal.
  5. Mismatched digest: 409 plus refreshed preview; no provider or journal.
  6. Matching digest: call `handlers::run_preflight`; return 202 with operation id and scrubbed report.

- `POST /v1/admin/projects/{id}/delivery/stage`
  - Accepts only `confirm`, `plan_digest` and required `confirm_operation_id` string.
  - Strictly parse the operation id as a non-negative `i64`; reject malformed values without echo.
  - Descriptor binds revision and canonical operation id.
  - Call `handlers::run_stage`; Core independently verifies project, kind, revision and healthy state.

- `POST /v1/admin/projects/{id}/delivery/promote`
  - Accepts only `confirm`, `plan_digest` and required 40-hex `confirm_revision`.
  - Descriptor binds registered and supplied revisions.
  - Call `handlers::run_promote`; Core independently refuses stale, missing or unhealthy prerequisites.

- `POST /v1/admin/projects/{id}/delivery/hermora-retry`
  - Accepts only `confirm`, `plan_digest`, required HTTP(S) `deployment_url` and required `secret_ref`.
  - `secret_ref` must be an environment-variable reference beginning with `env:`; it is never treated as a secret value.
  - Descriptor binds revision, URL and secret reference.
  - Call `handlers::run_hermora_retry`; Core independently requires a healthy promote row and validates the adapter envelope.

### Frontend

- Opening a workbench project loads delivery status after manifest/health/status.
- The delivery card shows:
  - phase and bound revision;
  - latest preflight, stage, promote and Hermora states;
  - the next staged confirmation in plain language;
  - loading, empty, unavailable, permission and error states.
- Generic catalog controls render the four mutations immediately below the card and refresh delivery status after success.
- All rendering uses `textContent`; no raw HTML, shell text, path field or credential field is introduced.

## 5. Contract and compatibility

- Route consts:
  - `GET /v1/admin/projects/{id}/delivery/status`
  - `POST /v1/admin/projects/{id}/delivery/preflight`
  - `POST /v1/admin/projects/{id}/delivery/stage`
  - `POST /v1/admin/projects/{id}/delivery/promote`
  - `POST /v1/admin/projects/{id}/delivery/hermora-retry`
- Catalog parameters:
  - preflight: none.
  - stage: `confirm_operation_id:string:required`.
  - promote: `confirm_revision:string:required`.
  - Hermora: `deployment_url:string:required`, `secret_ref:string:required`.
- All mutation rows are `remote_write`, confirm-required and digest-bound.
- Response shapes are additive; bearer responses and CLI output are unchanged.

## 6. Failure and boundary policy

| Case | Behavior |
|---|---|
| Missing/invalid session | 401 before Core |
| Non-JSON mutation | 415 before Core |
| Path-bearing id | 400 `admin-invalid-project-id`, no echo |
| Unmanaged id | 404 `admin-project-unmanaged` |
| Missing/invalid provider | 503 `delivery-unavailable`, journaled `failed` when Core reserves a row |
| Unhealthy provider response | Real failed state, never success |
| Missing/stale preflight evidence | Typed `delivery-invalid` for an unrecorded operation, or `delivery-conflict` for a stale/unhealthy prerequisite; no provider call |
| Missing/stale stage | 409 `delivery-conflict`, no provider call |
| Missing/stale promote | 409 `delivery-conflict`, no provider call |
| Missing/unhealthy deployment for Hermora | 409 `delivery-conflict`, no adapter call |
| Malformed confirmation field | 400 `delivery-invalid`, no echo |
| Credential-shaped adapter output | Typed refusal without echo |
| No confirm | 200 preview plus digest, no write |
| Digest mismatch | 409 plus refreshed preview, no write |
| Absolute path or secret in response | Scrubbed before return |

## 7. Verification oracle

- New `tests/forge_web_project_delivery_contract.rs` covers anonymous/non-JSON refusal, side-effect-free status, hostile/unmanaged ids, missing/stale prerequisites, preview/mismatch behavior, hazardous-field rejection, failed-provider honesty, unhealthy-stage promotion refusal, Hermora gating, the full stubbed run and catalog agreement.
- New Chromium drive signs in through the shipped login page, opens the fixture project, verifies draft delivery status, previews/confirms all four staged actions, verifies the final connected phase, checks keyboard focus, and asserts no absolute fixture path is rendered.
- `forge_web_command_catalog_contract`, `delivery_contract`, `delivery_cross_surface`, publish/deployment/release/action/workbench/admin/catalog/frontend/portal suites, `--bin forge` and `--lib api::` remain green.
- A task box is checked only with command output for its assertion.

## 8. Decision ledger

- **Resolved:** all staged verbs ship together because they share revision binding and Core state.
- **Resolved:** each mutation uses single-request POST preview/confirm rather than separate plan routes; status has its own read route.
- **Resolved:** `confirm_operation_id` is catalogued as a string and strictly parsed as `i64`, keeping the catalog scalar vocabulary closed.
- **Resolved:** Hermora retries may use an explicit environment-variable reference but never a secret value.
- **Resolved:** the workbench gains one delivery status card; mutations reuse generic controls rather than bespoke per-verb widgets.
- **Blockers:** none.
