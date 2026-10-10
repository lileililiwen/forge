# Forge handoff

## Current state

### web-assurance-browser delivered and archived (2026-10-10)

`web-assurance-browser` closes audit gap 7: operators can now browse the
seventeen assurance reads (`spec list|inspect|route`, `remediate
scan|diff`, `describe list|show`, `classify list|show`, `contract
list|inspect|emit`, `governance list|status|inspect`, `analytics
metrics`, `studio preview` read) from the projects view instead of
dropping to the CLI. Implemented, verified and archived as
`openspec/changes/archive/2026-10-10-web-assurance-browser`, promoting
canonical `web-assurance-browser` +2 requirements (no `--skip-specs`;
archiver-stamped TBD Purpose replaced with a source-backed sentence):

- **Seventeen read-only typed admin GET routes** (new
  `src/api/admin/assurance.rs`, reusing `deploy_id_gate`, `guarded`,
  `cors`, `error`, `scrub_*`): `GET /v1/admin/contracts` (list) +
  `.../contracts/{family}` (inspect) over the vendored contract
  catalog; project-bound `GET .../projects/{id}/specs` (list via
  `spec::list_specs`), `.../specs/{spec}` (inspect via
  `spec::read_spec` with CLI-parity prefix match), `.../spec/route?
  finding=` (`spec::route_finding` with CLI-parity source, never
  apply), `.../remediate/scan` (`remediation::scan`),
  `.../remediate/diff?finding=&pack=` (server-side `build_plan` +
  `diff`, no `--plan` file), `.../describe|classify/proposals` (list)
  + `.../proposals/{proposal}` (show) via `semantic::decide::list|
  read`, `.../contracts/emit?family=` (gate-result/readiness/
  release-evidence-latest/capability; job-outcome + audit-event +
  empty sources answer honest `unavailable-with-reason`),
  `.../governance` (`list_providers`), `.../governance/status` +
  `.../governance/inspect` (`evaluate_project` only — never
  `check_project`; enabled external providers answer
  `unavailable-with-reason`, adapters never run),
  `.../analytics/metrics?window_days=` (registry-only
  `aggregate_project_metrics` with empty externals; no provider probe,
  no summary write, no journal), `.../studio/preview`
  (`load_session` + `envelope_from_session`; none → `state: none`,
  never start/stop/spawn). Session-gated; hostile ids are static
  typed `400`s without echo; unmanaged ids typed `404`. No write, no
  provider, no adapter, no shell, no journal row, no browser-supplied
  path on any path; every response carries `contract:
  API_CONTRACT_VERSION` plus the registry contract where one exists.
- **Routing:** the nine assurance sections ride the shared
  `Route::AdminCreation { registry, item, action }` validated-key
  triple (creation-catalog precedent; disjoint keys) — no new variant,
  no new `router.rs`/permission/authorize/exhaustiveness arm
  (`router.rs` constant 999/1000; `deploy.rs` constant 991 via one
  `dispatch_creation` branch); `route_assurance` matcher beside the
  handlers via the `route_beside` fold; seventeen
  `ROUTE_ADMIN_*` constants; beside-handler `assurance_error_status`
  mapping for the reachable user-error codes.
- **Catalog:** the seventeen `NotYetWeb` rows convert to
  `web_at(Read, route, caps)` (caps verbatim; `studio preview` keeps
  its `LocalWrite` risk with the read-only restriction documented);
  count stays 234 (conversion); `problems()` empty. All writes stay
  where they are (`spec generate|apply`, `remediate plan|apply`,
  `classify apply|approve|reject` already web, untouched; `describe
  suggest|approve|reject`, `classify suggest|derive`, `contract
  validate`, `governance use`, `analytics inspect`, `studio
  spec|refine` CLI-or-existing-web, no new write route).
- **Frontend:** projects view gains a read-only "Assurance" card after
  the creation-catalog browser (registry picker + cached client-side
  search, list/inspect/route-or-scan-or-diff-or-emit-or-metrics-or-
  preview rendering, CLI-only remainder derived from the loaded
  command catalog as badges with reasons, honest
  unavailable-with-reason states; explicit reads only, `role=status`
  results, error-summary focus, native controls; no new dependency,
  no `innerHTML`).
- **Contract tests:** new
  `tests/web_assurance_browser_contract.rs` (12 tests: auth, gates,
  contract list/inspect incl. typed refusals, spec list/inspect/route
  incl. journal-free, scan/diff incl. server-side rebuild parity,
  describe/classify lists/shows, emit honest gaps, governance
  local + external-unavailable + no-persist, analytics clamps +
  no-probe-no-write, studio none-envelope, scrub, hostile/unknown,
  catalog pins for all 17 rows + 17 CLI-or-web leftovers) plus pin
  updates in `src/api/command_catalog/catalog.rs` (web vec +17, row
  order corrected to `rows()` order) and
  `tests/forge_web_command_catalog_contract.rs` (web-route +17 and
  web-id +17 allowlists).

Evidence:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | 0 errors; pre-existing warnings only (`ShareSurface` unused import) |
| `cargo test --test web_assurance_browser_contract` (new) | **12 passed / 0 failed** |
| `cargo test --lib` | **1213 passed / 0 failed** (1 ignored) |
| spec/remediation/governance/analytics CLI suites | **9 / 9 / 23 / 17 passed / 0 failed**; studio preview suites **7 + 1 passed** |
| `cargo test --test forge_web_command_catalog_contract` | **8 passed + 1 pre-existing failure** (`cap` help-coverage gap, identical signature to the recorded pristine failure; no CLI file in this diff) |
| `cargo test --test forge_web_project_workbench_contract` | **10 passed + 1 pre-existing failure** (`innerHTML` pin at workbench caret; this diff's app.js hunks exclude that region, `git diff` shows 0 `innerHTML` lines) |
| `web_project_catalog_browser` / `web_release_deploy_history` / `web_agent_identity_readonly` / `web_creation_catalog_browser` / `portal_ui` / `forge_web_navigation` / `web_command_reference_browser` / `forge_web_maintainer_surface` | **10 / 9 / 10 / 10 / 21 / 9 / 6 / 5 passed** |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **101 passed / 0 failed** active and post-archive |
| `node scripts/check-spec-governance.mjs` | PASS post-archive (after TBD Purpose repair + pointer removal) |
| `git diff --check` | clean (cached + worktree) |
| live oracle (throwaway API:8766+web:4173, scratch `FORGE_REGISTRY` with alpha fixture, `FORGE_ADMIN_PROJECTS_ROOT` set, real Chromium via bundled Playwright, script in `/tmp` — never committed) | **VERIFIED**: login → `/projects` → per-registry list (specs/remediate/contracts/governance/analytics/studio) → contract inspect → spec route → remediate scan → diff with live pack `baseline-service@0.9.0` → emit (readiness) → metrics → governance status → preview (`state: none`) → CLI-only badges; curl cross-check of all seventeen routes (16×200 + typed 404 unknown-spec) + anon gate + unmanaged 404; **zero JS console/page/network errors, zero failed requests** |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` (on the implementation tree) | **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (403/403 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| `openspec archive web-assurance-browser --yes` | archived as `2026-10-10-web-assurance-browser`, no `--skip-specs`; canonical `web-assurance-browser` +2; post-archive validate **101 passed / 0 failed**, names PASS |

Commits on `main`: implementation+specs+frontend+tests (commit 1 below)
+ this handoff (commit 2); nothing pushed; `openspec list`
reports no active changes and no `current_spec` pointer remains.

No active changes remain, so this handoff carries no `current_spec` pointer.

### web-creation-catalog-browser delivered and archived (2026-10-10)

`web-creation-catalog-browser` closes audit gap 6: operators can now
browse the six pure creation catalogs (profiles, features,
components, UI patterns, standards, procedures) with list/inspect/
resolve, validate structured intents, and read per-project standard
check/diff plus persisted plan receipts from the projects view
instead of dropping to the CLI. Implemented, verified and archived
as `openspec/changes/archive/2026-10-10-web-creation-catalog-browser`,
promoting canonical `web-creation-catalog-browser` +3 requirements
(no `--skip-specs`; archiver-stamped TBD Purpose replaced with a
source-backed sentence):

- **Twenty read-only typed admin GET routes** (new
  `src/api/admin/creation.rs`, reusing `deploy_id_gate`, `guarded`,
  `cors`, `error`, `scrub_*`): `GET /v1/admin/creation/{registry}`
  (list) + `.../{id}` (inspect) + `.../resolve` (query-driven) for
  `profiles|features|components|ui-patterns` via
  `profile::{list_profiles, inspect_profile, resolve_profile}`,
  `feature::{feature_catalog, inspect_feature, resolve_plan}`,
  `component::{component_catalog, inspect_component,
  resolve_outcome}`, `ui_pattern::{ui_pattern_catalog,
  inspect_ui_pattern, resolve_outcome}`; list+inspect for `standards`
  (`all_packs`, `inspect_pack`) and `procedures`
  (`procedure_catalog`, `inspect_procedure`);
  `GET .../creation/intents/validate` (action/profile/require/forbid/
  constraint query keys mirroring the CLI flags, `validate_intent` +
  `intent_hash`, journal-free by design); `GET
  .../projects/{id}/standard/check|diff` (`check_snapshot`/
  `diff_snapshot` over the server-resolved directory, bodies
  scrubbed) and `GET .../projects/{id}/intent/plans` (plan ids only,
  never the CLI's absolute receipt path; absent dir → empty, not an
  error). Session-gated; hostile ids are static typed `400`s without
  echo (traversal adding a segment matches no route → 404);
  unmanaged ids typed `404`. No write, no provider, no adapter, no
  toolchain probe, no shell, no journal row, no browser-supplied path
  on any path; every response carries `contract:
  API_CONTRACT_VERSION` plus the registry contract where one exists.
- **Routing:** one `Route::AdminCreation { registry, item, action }`
  validated-key triple (portfolio `{kind}`/`{action}` precedent) with
  a `route_creation` matcher beside the handlers; the pre-table chain
  folds into a `route_beside` helper in `src/api/admin/mod.rs` so
  `router.rs` stays at 999/1000 lines (permission/short-circuit/
  exhaustiveness append to existing arms); one `deploy.rs` dispatch
  arm + one `handlers_core.rs` authorize arm; twenty
  `ROUTE_ADMIN_CREATION_*` constants; the six reachable user-error
  codes map to `400` in a `creation_error_status` helper beside the
  handlers (kept out of `err_status` for the same cap).
- **Catalog:** the twenty `NotYetWeb` rows convert to `web_at(Read,
  route, caps)` (caps verbatim); count stays 234 (conversion);
  `problems()` empty. `profile preflight`, `procedure validate`
  (native/file boundary), `component qualify`, `ui-pattern install`,
  `standard upgrade` (writes) and all other writes stay CLI-only with
  reasons; `intent resolve|apply` already web, untouched.
- **Frontend:** projects view gains a read-only "Creation catalog"
  card after the project-catalog browser (registry picker + cached
  client-side search, list/inspect/resolve-or-check/diff rendering,
  project-gated standard/intent panels, CLI-only remainder derived
  from the loaded command catalog as badges with reasons, honest
  unavailable-with-reason states; explicit reads only, `role=status`
  results, error-summary focus, native controls; no new dependency,
  no `innerHTML`).
- **Contract tests:** new
  `tests/web_creation_catalog_browser_contract.rs` (10 tests: auth,
  gates, six lists, inspects incl. live pack selector + typed
  refusals, four resolves incl. missing-subject, intent validate
  happy + malformed + journal-free, project-bound reads incl. scrub
  asserts, catalog pins for all 20 rows + 5 CLI-only leftovers) plus
  pin updates in `src/api/command_catalog/catalog.rs` (web vec +20)
  and `tests/forge_web_command_catalog_contract.rs` (web-route +20
  and web-id +20 allowlists).

Evidence:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | 0 errors; pre-existing warnings only (`ShareSurface` unused import) |
| `cargo test --test web_creation_catalog_browser_contract` (new) | **10 passed / 0 failed** |
| `cargo test --lib` | **1213 passed / 0 failed** (1 ignored) |
| profile/feature/component/ui-pattern/standard/procedure/planner CLI suites | **132 passed / 0 failed** across 12 binaries |
| `cargo test --test forge_web_command_catalog_contract` | **8 passed + 1 pre-existing failure** (`cap` web-id gap; no CLI file in this diff, signature byte-identical to the recorded pristine failure) |
| `cargo test --test forge_web_project_workbench_contract` | **10 passed + 1 pre-existing failure** (`innerHTML` pin at workbench caret; this diff's app.js hunks exclude that region) |
| `cargo test --test web_project_catalog_browser_contract` / `web_release_deploy_history_contract` / `web_agent_identity_readonly_contract` / `portal_ui_contract` / `forge_web_navigation_contract` / `web_command_reference_browser_contract` / `forge_web_maintainer_surface_contract` | **10 / 9 / 10 / 21 / 9 / 6 / 5 passed** |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **100 passed / 0 failed** active and post-archive |
| `node scripts/check-spec-governance.mjs` | PASS post-archive (after TBD Purpose repair + pointer removal) |
| `git diff --check` | clean (cached + worktree) |
| live oracle (throwaway API:8766+web:4173 default ports, scratch `FORGE_REGISTRY` with alpha fixture, `FORGE_ADMIN_PROJECTS_ROOT` set, real Chromium via bundled Playwright, script in `/tmp` — never committed) | **VERIFIED**: login → `/projects` → list 6 profiles → inspect rust-web → resolve audit-action (plan ids render) → validate intent hash → check absent snapshot → plans empty → CLI-only badges for preflight/qualify/install/upgrade/validate; curl cross-check of all twenty routes (19×200 + typed 400 diff-without-snapshot) + anon 401; **zero JS console/page/network errors, zero failed requests** |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` (on committed tree) | **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (402/402 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. Mid-task note: an interim `cargo fmt` split same-line router arms (1007 lines, gate source-file-size fail); repaired via the `route_beside` fold + beside-handler status mapping before archive, re-verified green. |
| `openspec archive web-creation-catalog-browser --yes` | archived as `2026-10-10-web-creation-catalog-browser`, no `--skip-specs`; canonical `web-creation-catalog-browser` +3; post-archive validate **100 passed / 0 failed**, names PASS |

Commits on `main`: implementation+specs+frontend+tests (`ba29ff9`)
+ this handoff (commit 2); nothing pushed; `openspec list`
reports no active changes and no `current_spec` pointer remains.

No active changes remain, so this handoff carries no `current_spec` pointer.

### web-agent-identity-readonly delivered and archived (2026-10-10)

`web-agent-identity-readonly` closes audit gap 5: operators can now
read per-project agent sessions, identity sessions and the validated
identity configuration from the Workbench instead of dropping to the
CLI. Implemented, verified and archived as
`openspec/changes/archive/2026-10-10-web-agent-identity-readonly`,
promoting canonical `web-agent-identity-readonly` +6 requirements (no
`--skip-specs`; archiver-stamped TBD Purpose replaced with a
source-backed sentence):

- **Five read-only typed admin GET routes** (new
  `src/api/admin/agent_identity.rs`, reusing `deploy_id_gate`,
  `guarded`, `cors`, `error`, `scrub_*`): `GET .../{id}/agents`
  (`agent::list_sessions`), `GET .../agents/{session_id}`
  (`agent::status_for`, unknown → typed 404, `project_path`
  projected out, `live: null` + CLI-only note — `live_runtime_status`
  never called so no adapter subprocess spawns — plus spawn-free
  `probe_provider` availability), `GET .../identity/config`
  (`IdentityConfig::from_manifest_opt`, unconfigured → typed 404,
  invalid → typed `identity-invalid` via the new `err_status` 400
  arm, no provider contacted), `GET .../identity/sessions`
  (`identity::list_sessions`), `GET
  .../identity/sessions/{session_id}` (`identity::load_session`,
  unknown → typed 404, sibling-owned → typed 403 cross-project
  refusal). Session-gated; hostile ids, bad agent/identity session
  ids (kebab-danger / non-hex gates) are typed `400`s without echo;
  unmanaged ids typed `404`. No write, no challenge, no mint/revoke,
  no provider, no shell on any path; every response carries
  `contract: API_CONTRACT_VERSION` with no path or secret material.
- **Routing:** three `Route` variants (list/inspect folded on
  `Option`) + `route_agent_identity` matcher tried before the main
  table (kept beside the handlers so `router.rs` lands at 998/1000
  lines), permission/authorize/dispatch/exhaustiveness threading,
  five `ROUTE_ADMIN_*` constants, six-segment OPTIONS arm widened in
  place to six-or-more (covers the 7-segment identity inspect with
  no added line).
- **Catalog:** `agent.status|list`,
  `identity.validate-config|session-list|session-inspect` convert
  `NotYetWeb` → `web_at`; `identity.session-terminate` converts
  leaf → `cli_only` (a write the browser must never trigger); count
  stays 234 (conversion); `identity.build-challenge` stays
  `NotYetWeb` (persist + `code_verifier` secret boundary, OIDC
  round-trip non-goal); every lifecycle/secret/transport verb stays
  `cli_only`.
- **Frontend:** Workbench gains a read-only "Agent & identity" card
  reusing `#workbench-project` (explicit reads only, no auto-load;
  list buttons, inspect inputs, config read; `role=status` results,
  error-summary focus, native controls; no new dependency, no
  `innerHTML`).
- **Contract tests:** new
  `tests/web_agent_identity_readonly_contract.rs` (10 tests: auth,
  gates, lists, inspects incl. cross-project, config
  validated/unconfigured/invalid, scrub, no-write-on-read, catalog +
  frontend pins) plus pin updates in
  `src/api/command_catalog/catalog.rs` (web vec) and
  `tests/forge_web_command_catalog_contract.rs` (web-route + web-id
  allowlists).

Evidence:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | 0 errors; pre-existing warnings only (`ShareSurface` unused import) |
| `cargo test --test web_agent_identity_readonly_contract` (new) | **10 passed / 0 failed** |
| `cargo test --lib` | **1213 passed / 0 failed** (1 ignored) |
| `cargo test --test forge_web_command_catalog_contract` | **8 passed + 1 pre-existing failure** (`cap` web-id gap, byte-identical on pristine tree via `git stash -u` rerun) |
| `cargo test --test web_release_deploy_history_contract` / `web_project_catalog_browser_contract` / `forge_web_maintainer_surface_contract` / `portal_ui_contract` | **9 / 10 / 5 / 21 passed** |
| `cargo test --test forge_web_project_workbench_contract` | **10 passed + 1 pre-existing failure** (`innerHTML` pin, byte-identical on pristine tree via `git stash -u` rerun) |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **99 passed / 0 failed** pre- and post-archive |
| `node scripts/check-spec-governance.mjs` | PASS post-archive (after TBD Purpose repair + pointer removal) |
| `git diff --check` | clean (cached + worktree) |
| live oracle (throwaway API+web on default ports, scratch `FORGE_REGISTRY`, real Chromium via `tests/browser` Playwright, script in `/tmp` — never committed) | **VERIFIED**: login → `/workbench?project=live-probe` → list agents (live-agent) → inspect (recorded-only note) → list/inspect identity (subject user-1) → config read (okta); curl cross-check of all five routes (200 + envelope), anon 401, unknown 404; **zero attributable JS errors** (two 409 console resource errors come from pre-existing workbench auto-loads `workspace/candidates` + `delivery/status`, untouched by this change; my five card GETs all 200) |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` (on committed tree) | **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (401/401 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| `openspec archive web-agent-identity-readonly --yes` | archived as `2026-10-10-web-agent-identity-readonly`, no `--skip-specs`; canonical `web-agent-identity-readonly` +6; post-archive validate **99 passed / 0 failed**, names PASS |

Commits on `main`: implementation+specs+frontend+tests (`d32d028`)
+ this handoff (commit 2); nothing pushed; `openspec list`
reports no active changes and no `current_spec` pointer remains.

No active changes remain, so this handoff carries no `current_spec` pointer.

### web-release-deploy-history delivered and archived (2026-10-10)

`web-release-deploy-history` closes audit gap 4: operators can now
browse persisted release/deploy history from the dashboard instead of
dropping to the CLI. Implemented, verified and archived as
`openspec/changes/archive/2026-10-10-web-release-deploy-history`,
promoting canonical `web-release-deploy-history` +6 requirements (no
`--skip-specs`; archiver-stamped TBD Purpose replaced with a
source-backed sentence):

- **Five read-only typed admin GET routes** (new
  `src/api/admin/history.rs`, reusing `deploy_id_gate`, `guarded`,
  `cors`, `error`, `scrub_*`): `GET
  .../{id}/releases` (`release::engine::list_releases`), `GET
  .../releases/{release_id}` (`read_release`, unknown → typed 404),
  `GET .../{id}/deploys` (`deploy::engine::list_deploys` with the
  absolute `state_path` projected out), `GET
  .../deploys/{deploy_id}` (`read_deploy`, unknown → typed 404),
  `GET .../{id}/deploy/status` (`operations_for_project` filtered
  to the CLI's `publish|deploy|publish.github` set, `?limit=`
  default 20 clamped 1..=100). Session-gated; hostile ids, bad
  inspect ids (`/`, `\`, `..`, `%` rejected without echo) and bad
  limits are typed `400`s; unmanaged ids typed `404`. No write, no
  journal row, no provider, no adapter, no shell on any path; every
  response carries `contract: API_CONTRACT_VERSION` and is scrubbed
  of the project dir.
- **Routing:** five `Route` variants + `route_history` matcher tried
  before the main table (kept beside the handlers so `router.rs`
  stays at 991/1000 lines), permission/short-circuit/exhaustiveness
  threading, five `ROUTE_ADMIN_*` constants.
- **Catalog:** `release.list|inspect`,
  `deploy.list|inspect|status` convert `NotYetWeb` → `web_at`;
  count stays 234 (conversion); `deploy.observe` untouched.
- **Frontend:** Delivery gains a read-only "Release & deploy
  history" card reusing `#delivery-project` (list buttons, inspect
  inputs, status read; `role=status` results, error-summary focus,
  native controls; no new dependency, no `innerHTML`).
- **Contract tests:** new
  `tests/web_release_deploy_history_contract.rs` (9 tests: auth,
  gates, lists, inspects, status filter, limit, scrub, catalog +
  frontend pins) plus pin updates in
  `src/api/command_catalog/catalog.rs` (web-id vec) and
  `tests/forge_web_command_catalog_contract.rs` (web-route + web-id
  allowlists).

Evidence:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | 0 errors; 3 pre-existing warnings only |
| `cargo test --test web_release_deploy_history_contract` (new) | **9 passed / 0 failed** |
| `cargo test --lib` | **1212 passed + 1 attributable pin failure fixed in-tree** (`web_rows_only_point_at_implemented_routes` hardcoded web vec; updated with the 5 rows, then green on rerun) |
| `cargo test --lib api::command_catalog` (closeout) | **7 passed / 0 failed** |
| `cargo test --test forge_web_project_release_contract` / `forge_web_project_deployment_contract` | **8 / 8 passed** |
| `cargo test --test portal_ui_contract` / `forge_web_project_delivery_contract` / `web_command_reference_browser_contract` / `forge_web_maintainer_surface_contract` / `web_project_catalog_browser_contract` | **21 / 12 / 6 / 5 / 10 passed** |
| `cargo test --test forge_web_command_catalog_contract` | **8 passed + 1 pre-existing failure** (`cap` web-id gap, byte-identical on pristine tree via `git stash -u` rerun; the second failure was the attributable web-id pin, fixed in-tree) |
| `cargo test --test forge_web_project_workbench_contract` | **10 passed + 1 pre-existing failure** (`innerHTML` pin: both uses pre-date this change, count 2 on both trees) |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **98 passed / 0 failed** pre- and post-archive |
| `git diff --check` | clean (cached + worktree) |
| live oracle (throwaway API+web, scratch `FORGE_REGISTRY`, real Chromium via `tests/browser` Playwright driving system Chrome, script in `/tmp` — never committed) | **VERIFIED**: login → `/delivery` → select p1 → List releases (rel-live/2.0.0) → inspect rel-live → list/inspect dep-live → status read (honest `empty`); missing-project refusal focuses the error summary; curl cross-check of all five routes (200 + envelope) and anon 401; **zero attributable JS errors** |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` (on committed tree) | **blocked, 0 attributable** — pass: build, placeholder-threshold, product-code-boundary, repository, security, source-file-size (400/400 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries); fail pre-existing: governance-quality (the single in-scope error is the `web-project-catalog-browser` canonical TBD Purpose left by commit `32bd8fc`, byte-identical on the pristine tree via node-checker rerun — this change's own promoted Purpose was repaired source-backed). Remediation: replace that TBD Purpose with a source-backed sentence; warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| `openspec archive web-release-deploy-history --yes` | archived as `2026-10-10-web-release-deploy-history`, no `--skip-specs`; canonical `web-release-deploy-history` +6; post-archive validate **98 passed / 0 failed**, names PASS |

Commits on `main`: implementation+specs+frontend+tests (`1011117`)
+ this handoff (commit 2); nothing pushed; `openspec list`
reports no active changes and no `current_spec` pointer remains.

### web-portfolio-completion delivered and archived (2026-10-10)

`web-portfolio-completion` is implemented, verified and archived as
`openspec/changes/archive/2026-10-10-web-portfolio-completion`,
promoting canonical `web-portfolio-completion` +5 requirements (no
`--skip-specs`; archiver-stamped TBD Purpose replaced with a
source-backed sentence): the Portfolio view now wires every
Forge-owned metadata verb with preview→confirm→apply semantics. Reads
reuse `GET /v1/admin/portfolio`, `/evidence` and `/{id}` (show);
`GET /v1/admin/portfolio/{id}/{kind}` grows `evidence`-only to
`evidence|tags|relations|reviews|goals` from existing typed registry
reads (goals filtered to membership) — no provider probed on any
read. All seven write entries require `confirm: true`, else `409
portfolio-confirm-required` with the current-state preview and
`effect: "none"`: existing `tags|relations|reviews|goals` actions
plus new `POST …/tags/remove`, `POST …/relations/remove`
(idempotent `removed` flag) and `POST …/evidence/import`
(append-only `SnapshotWrite` via Core validation; `POST …/evidence`
stays the `403 portfolio-source-owned` edit refusal). No digest
binding (no manifest exists to hash — confirm binds to the reviewed
key plus echoed preview, recorded in design §4) and no journal rows
(the portfolio store has no audit table; `forge-owned-write` envelope
retained). Fourteen `portfolio.*` catalog leaves convert
`NotYetWeb` → `web_at` (count stays 234 — conversion, not addition);
write half split verbatim into `src/api/portfolio_writes.rs` via
`#[path]` so both files stay under the source-file-size cap (a
`portfolio/` directory split was rejected: the gate scan enumerates
HEAD-tracked paths and reads a deleted-then-uncommitted path as
missing). Frontend keeps the existing card/styles/a11y
(`role=status` previews, error-summary focus, per-action confirm
checkboxes, read-only evidence); two selection-preservation fixes
found by the live oracle (fleet refresh kept the project select,
list refresh keeps the tag-remove pick). No new dependency, no
`innerHTML`, no inline script.

Evidence:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | 0 errors; 3 pre-existing warnings + 1 pre-existing unused-import warning (`portfolio/share/validation.rs`, untouched) |
| `cargo test --test forge_web_portfolio_controls_contract` | **16 passed / 0 failed** (11 existing incl. confirm-updated writes + 5 new: confirm refusal, tag/relation remove round-trips, membership-filtered lists, append-only import) |
| `cargo test --lib` | **1213 passed / 0 failed** (incl. catalog `234` count + web-route pins) |
| `cargo test --test forge_web_command_catalog_contract` | **7 passed + 2 pre-existing failures** (graduation/remediate web-id gap, byte-identical on pristine tree via `git stash -u` rerun; portfolio pins updated and green) |
| `cargo test --test forge_web_project_workbench_contract` | **10 passed + 1 pre-existing failure** (`innerHTML` pin, same test as prior entries) |
| `cargo test --test portfolio_ui_contract` | **0/20 pre-existing failure** (legacy form-post surface, byte-identical on pristine tree via `git stash -u` rerun) |
| `cargo test --test portal_ui_contract` / `forge_web_navigation_contract` / `web_command_reference_browser_contract` / `forge_web_maintainer_surface_contract` | **21 / 9 / 6 / 5 passed** |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **96 passed / 0 failed** pre- and post-archive |
| `git diff --check` | clean (cached + worktree) |
| live oracle (throwaway API+web, scratch `FORGE_REGISTRY`, real Chromium via bundled Playwright driving system Chrome, script in `/tmp` — never committed) | **VERIFIED**: login → `/portfolio` → show, tag add (unticked client refusal verified) → remove, relation add → remove, review, goal save → link, evidence import; curl cross-check of all 13 routes incl. 409 path; **zero attributable JS errors** (only pre-existing favicon 404 + rootless workspace-candidates 409, both documented in prior entries) |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (398/398 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| `openspec archive web-portfolio-completion --yes` | archived as `2026-10-10-web-portfolio-completion`, no `--skip-specs`; canonical `web-portfolio-completion` +5; post-archive validate **96 passed / 0 failed**, names PASS |

Commits on `main`: implementation+specs+frontend+tests (`9ab0dad`)
+ this handoff (commit 2); nothing pushed; `openspec list`
reports no active changes and no `current_spec` pointer remains.

### workbench-health-latency delivered and archived (2026-10-10)

`workbench-health-latency` is implemented, verified and archived as
`openspec/changes/archive/2026-10-10-workbench-health-latency`,
promoting canonical `workbench-health-latency` +3 requirements (no
`--skip-specs`): the detail GET keeps route/shape/version/journal
behavior with a fast local `health` projection (`run_doctor` with no
policy outcome + appended `policy-deferred` finding; locally-clean
reports new state `deferred`, never `healthy`); new confirm-free,
journal-free `POST /v1/admin/projects/{id}/health/refresh` runs the
exact live DriftWatch + doctor pass the GET ran before; the workbench
health card renders `deferred` (`HEALTH_LABELS`/`HEALTH_BADGE`) with a
**Run full health check** control (progress state via `refreshHealth`,
re-renders the card) on the `policy-deferred` row and never a remediate
shortcut; sentinel-oracle
`tests/workbench_health_latency_contract.rs` (5 tests) + workbench
contract `deferred` pin. One deliberate deviation from `design.md` §1:
the refresh route carries no catalog row — direct-called route, same
precedent as the maintain GET — so `IMPLEMENTED_WEB_ROUTES` gains the
route while the catalog count stays 234.

Evidence:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | 0 errors; 3 pre-existing warnings only |
| `cargo test --test workbench_health_latency_contract` (new) | **5 passed / 0 failed** (GET leaves sentinel untouched + `deferred`; refresh touches it + full document, no deferred finding; local `issues` without checker; refresh 401-gated; frontend token pins) |
| `cargo test --test forge_web_project_workbench_contract` | **10 passed + 1 pre-existing failure** (`innerHTML` pin, identical on the pristine tree, recorded earlier) |
| `cargo test --test forge_web_maintainer_surface_contract` | **5 passed / 0 failed** |
| `cargo test --test forge_web_command_catalog_contract` | **7 passed + 2 pre-existing failures** (`cap`-coverage gap, identical on the pristine tree) |
| `cargo test --test portal_ui_contract` | **21 passed / 0 failed** |
| `cargo test --lib doctor` / `policy` / `api` / `workbench` | **39 / 42 / 32 / 5 passed** (one transient policy failure, green on 3 retries) |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **95 passed / 0 failed** |
| `git diff --check` | clean |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (397/397 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| Closeout rerun (this session): `cargo fmt --check`, `cargo build`, `workbench_health_latency_contract`, `forge_web_project_workbench_contract`, `portal_ui_contract`, validate, `git diff --check` | fmt clean; build 0 errors + same 3 pre-existing warnings; latency **5 passed**; workbench **10 passed + 1 pre-existing** (`frontend_workbench_is_standalone_json_only_and_shell_free` `innerHTML` pin, confirmed same test/line as the pristine-tree record); portal_ui **21 passed**; validate **95 passed / 0 failed**; diff clean |
| Closeout `forge gate --dry-run` + `forge gate --timeout-secs 600` | plan rendered (9 required checks); **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (397/397); unresolved pre-existing: declared-verification + tests (`project-runtime` 60s adapter timeout, identical signature) |
| `openspec archive workbench-health-latency --yes` | archived as `2026-10-10-workbench-health-latency`, no `--skip-specs`; canonical `workbench-health-latency` +3; post-archive validate **95 passed / 0 failed**, names PASS, governance PASS after replacing the archiver-stamped TBD Purpose with a source-backed sentence |

Commits on `main`: implementation+specs+frontend+tests (`2693de5`)
+ progress handoff (`8584ec6`) + closeout archive+canonical-spec
(`466ee4c`) + this handoff (commit 2); nothing pushed; `openspec list`
reports no active changes and no `current_spec` pointer remains.

Manual dashboard verification (this closeout, live throwaway listeners
with a scratch `FORGE_REGISTRY`, sentinel checker, real Chromium via a
one-off playwright script since removed — never committed):

| Check | Result |
|---|---|
| `POST /v1/admin/session` (curl) | **200** `authenticated:true`, session cookie minted |
| `GET /v1/admin/projects/wb-manual` (curl, timed) | **200 in ~7ms** (was ~6.5s direct / ~27s browser); `health.state: deferred`, `policy-deferred` appended last, sentinel untouched, no journal row |
| `POST .../health/refresh` (curl, timed) | **200 in ~56ms**; sentinel touched (checker invoked), no deferred finding; state `issues` — honest: the empty sentinel report parses to a `driftwatch-policy unavailable` row, the same content the pre-change live GET returned for this fixture |
| Chromium: login → `/workbench?project=wb-manual` | **VERIFIED** — signed in through the shipped login page; project open **606ms**; card reads **Deferred** with **Run full health check** and no remediate shortcut; click shows **Running full check…** then re-renders the full card (**Has issues**); **zero JS console errors** |

### resync-ui-token-kit delivered and archived (2026-10-09)

`resync-ui-token-kit` moved the vendored UI token kit mirror from kit
revision `c740bd9` (0.1.0) to kit **0.2.0** at
`6bf3946fcb1a9d2294b13d0af4347f6cf0d8e566` (the follow-up handoff commit
changes no `ui/` bytes), archived as
`openspec/changes/archive/2026-10-09-resync-ui-token-kit` (promoting
canonical `scaffold-prewires-shared-layer` +1 requirement, no
`--skip-specs`):

- **Mirror bytes.** `kits/tokens/tokens.css`
  `720d0bc7…382b191` → `7ca6b7956b3f6b475254b4a5d0cf599724e4e1af5f0f88aba4fbccb9f38fc19b`;
  `kits/tokens/tokens.ts` `98dfd642…614f38` →
  `7e0633d3a18ad6039f160838412ddb5d3f30260fa8ef594d8b266f9e5d646c29`;
  `kits/scripts/verify-tokens.mjs` `29c7f0bf…37ed82` →
  `1cba69d465cc8ed639a958f87dda03241333576694c2942dd50d6c9de4866f4b`
  (re-derived from upstream `verify-ui.mjs`: provenance to `6bf3946…`,
  `--color-focus-ring` asserted, new `tokenValues`/`semanticColorValues`
  light+dark assertions). Byte-identity with the kit dist confirmed by
  `diff` + `sha256sum`.
- **Manifest.** `kits/manifest.json`: `revision` → `6bf3946…`, top-level
  `"version": "0.2.0"` added, per-file sha256 renewed for the three UI
  files, `synced_at` stamped; the 9 `feed/*.nupkg` entries untouched
  (dotnet kit stays 0.1.0). The `version` key is **modeled** in
  `KitManifest` (`#[serde(default)]`) because `forge kit pack`
  re-serializes the manifest and would silently drop an unmodeled field.
- **Compiled-in surfaces.** `src/kit/assets/verification.rs::token_assets()`
  digests renewed; `src/kit/registry.rs` `PLATFORM_UI_KIT_VERSION` →
  `"0.2.0"`; `src/contract/mod.rs` inventory row `"0.2.0"` and
  `inventory_agreement` now checks the UI-kit row against the compiled-in
  constant; `tests/kit_contract/{tokens_and_assets,upgrade}.rs` pins →
  `platform-ui-web@0.2.0`. `src/contract/mod.rs` exceeded the 1000-line
  cap after the edit; tests extracted verbatim to
  `src/contract/contract_tests.rs` per the `agent_tests.rs` convention.
- **Deferred (explicit non-goals).** The kit's new Tailwind artifacts
  (`dist/tailwind/preset.js`, `theme.css`) and the npm tarball
  `platform-design-tokens-0.2.0.tgz` (sha `e48284cb…` verified against
  the kit digest) are **not vendored**: no `forge/kits`, descriptor or
  scaffold consumer exists; `kits/feed` holds dotnet nupkgs only.

Evidence:

| Check | Result |
|---|---|
| vendored `verify-tokens.mjs` run co-located with the dist pair | **PASS** |
| `cargo test --test kit_contract` | **59 passed / 0 failed / 1 ignored** |
| `cargo test --lib` | **1213 passed** |
| `cargo test --lib contract` | **26 passed** (incl. `inventory_agreement`) |
| `forge contract list` | `kit PLATFORM_UI_KIT_VERSION 0.2.0` |
| `forge kit verify` | verified 9 feed files at `kits/feed` |
| `forge profile inspect react-web` | `kit: platform-ui-web@0.2.0` |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **96 passed / 0 failed** before archive; **95 passed / 0 failed** after |
| `git diff --check` | clean (archiver's blank-line-at-EOF in the promoted spec repaired) |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 500` | **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (397/397 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| `openspec archive resync-ui-token-kit -y` | archived as `2026-10-09-resync-ui-token-kit`; canonical `scaffold-prewires-shared-layer` +1 requirement |

Commits on `main`: mirror+manifest+verifier+surfaces+tests+archive+canonical
spec (commit 1) + this handoff (commit 2); nothing pushed; this cycle's
`current_spec` pointer is removed (the active `workbench-health-latency`
change belongs to another session and was never touched or claimed here).

### web-lifecycle-execution delivered and archived (2026-10-09)

`web-lifecycle-execution` closes the three proposal gaps on `main` as one
change, implemented, verified and archived as
`openspec/changes/archive/2026-10-09-web-lifecycle-execution`
(promoting canonical `web-lifecycle-execution` +6, no `--skip-specs`):

- **Idea execution (backend + web).** New session-gated admin routes
  `POST /v1/admin/graduation/preview` (artifact JSON text → validated
  brief + `plan_digest`, no write) and
  `POST /v1/admin/graduation/import` (same artifact + `profile` +
  optional `id` + `confirm:true` + `plan_digest` → stale-digest refused
  409 with fresh preview, else same in-process
  `parse_artifact`/`validate_graduation`/`build_proposal`/`adopt_graduation`
  as the CLI, destination joined server-side under
  `FORGE_ADMIN_PROJECTS_ROOT`, journal row `graduation.import`). Web
  `#wb-idea-entry` is a real form (artifact textarea, profile select, id
  override, Preview → confirm checkbox → Run, `role=status` result,
  error-summary focus, journal evidence reload). No shell, no browser path.
- **Maintain execution (backend + web).** New session-gated per-project
  routes `POST /v1/admin/projects/{id}/intent/resolve` (typed intent
  fields → plan + `plan_digest`, no receipt) and `.../intent/apply`
  (`confirm:true` + `plan_digest` → recompute, stale refused 409 with
  fresh plan, else `write_plan_receipt` + `apply_plan`, journal
  `intent.apply`); `POST .../remediate/plan` (`finding` → plan +
  `plan_digest`, no write, target resolved server-side) and
  `.../remediate/apply` (digest-bound, journal `remediate.apply`). The
  terminal `forge remediate plan --finding <id>` string stays where shown.
  Delivery keeps approve/publish/stage; publish success additionally
  records journal row `delivery.next-idea` via
  `POST .../delivery/next-idea` and renders it in `#delivery-next-idea`
  plus the workbench operations table.
- **Studio refine wiring (web only).** Refine confirm reuses existing
  `POST .../studio/refine` (admin-gated wrapper); the card shows the
  returned `spec_revision`/`app_revision` bump in a `role=status` result
  and reloads journal evidence. No new endpoint.
- **Catalog.** `graduation.preview|import`, `intent.resolve|apply`,
  `remediate.plan|apply`, `studio.spec|refine` → `web` with typed routes
  and params; pinned `tests/browser` playwright 1.63.0 reused, no new dep.

Click-oracle coverage (`tests/browser/lifecycle-rail-check.mjs` driven by
`tests/lifecycle_rail_browser.rs`, exit 2 → UNVERIFIED): every lifecycle
confirm clicks end to end in rail order on a throwaway registry — idea
invalid→preview→confirm-required→confirm→run (import + journal row),
scaffold `new` invalid→preview→confirm→run (creates + journal),
studio spec save (r0 + `studio.spec.save` row), studio refine (r1→r2 bump
+ `studio.refine` row), upgrade plan (gate dry-run analog,
`role=status`), delivery approve confirm-refused (summary focus),
maintain refresh (re-renders), remediate plan bogus refused, remediate
apply bogus refused, intent resolve preview, intent apply
preview→confirm→run (`role=status` + `intent.apply` row) — with zero JS
console errors, error-summary focus on every invalid submit,
`role=status` updates, one journal row per success, and a screenshot per
step under `target/lifecycle-rail-shots/`.

Bugs the oracle found and fixed:

- **Refine revision-bump / detached nodes (prior session).** The confirmed
  response landed on rebuilt card nodes after `loadWorkbenchDetail` and
  never rendered; success now re-targets the live card's
  `.wb-plan-result` and surfaces the `spec_revision`/`app_revision` line.
- **Textarea params (prior session).** `spec`/`artifact_json`/`request`
  rendered as single-line inputs stripping newlines so pasted YAML/JSON
  could never parse; they render as textareas now.
- **In-flight rebuild guards (prior session).**
  `renderProjectActions`/`renderManagementActions`/`renderMaintainActions`
  rebuilt mid-preview wiping typed fields, held digests and results; they
  keep live cards while any card is open or shows a result.
- **Intent/remediate apply preview (this session).** The apply endpoints
  required `confirm:true` with no preview mode, so the generic card's
  Preview could never arm Run (button stayed disabled, oracle timed out);
  missing-confirm now returns 200 preview + `plan_digest`, Run confirms
  with the digest as before.
- **Console-error guard (this session).** Expected typed refusals
  (400/409) log Chromium "Failed to load resource" network lines; the
  harness ignored exactly those and still fails on any real JS error.
- **Source-file-size (this session).** New `src/api/lifecycle_exec.rs`
  reached 1316 lines over the 1000 cap; split verbatim into
  `src/api/lifecycle_exec/` (`mod` + 5 submodules, all ≤330 lines,
  `pub` + `pub use` keeping every `lifecycle_exec::` path stable) plus an
  unused-import removal in `rows_project.rs`.

Evidence:

| Check | Result |
|---|---|
| `cargo test --test portal_ui_contract` | **21 passed / 0 failed** |
| `cargo test --test lifecycle_rail_browser` | **1 passed** — `VERIFIED: lifecycle-rail-check ok` (rail shape + Next, step coexistence load+click+reload, bogus ignored, roving arrows/Home/End, copy-CLI exact, contrast AA, idea entry links, cap badge/filter, demo URLs, all lifecycle clicks above console-error-free, 390px no overflow) |
| `cargo test --test web_lifecycle_execution_contract` | **9 passed / 0 failed** (preview/apply happy paths, missing-confirm refused, stale-digest refused with fresh preview, journal rows, catalog routes, frontend tokens) |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **95 passed / 0 failed** active; **95 passed / 0 failed** after archive |
| `git diff --check` | clean |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 500` | **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (396/396 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| `openspec archive web-lifecycle-execution --yes` | archived as `2026-10-09-web-lifecycle-execution`, no `--skip-specs`; canonical `web-lifecycle-execution` +6; `openspec list` leaves only `github-gh-fallback-register` active |

Commits on `main`: implementation+specs+UI+playwright+tests (commit
1) + this handoff (commit 2); nothing pushed; `current_spec`
advances back to `github-gh-fallback-register` (active, unarchived).

### flywheel-plugin-cap delivered and archived (2026-10-09)

`flywheel-plugin-cap` closes the three absorbed gaps as one change,
implemented, verified and archived as
`openspec/changes/archive/2026-10-09-flywheel-plugin-cap`
(promoting `portal-web-ui` +4 and `forge-publish-plugin-orchestration`
+4, no `--skip-specs`):

- **Flywheel ends in web.** Workbench rail step 0 gains `#wb-idea-entry`
  (graduation preview/import CLI + studio spec entry link,
  `?project=`+`?step=` preserving); delivery publish success renders
  `#delivery-next-idea` (Next-idea prompt + `/workbench?project=<id>&step=idea`
  link + maintain refresh shortcut firing `#wb-maintain-refresh`);
  canonical `docs/flywheel-demo.md` walks `hookit` through
  idea→scaffold→gate→publish→maintain with exact CLI + web URLs.
- **Plugin reinforcement.** `src/plugins/mod.rs` `CAPABILITIES` 6→11
  (+`gate,quality,agent,contract,analytics`), `PluginKind`
  +`Gate,Quality,Agent,Contract` (serde lowercase; `Metadata|Delivery`
  unchanged, unknown stays `Invalid`); builtin table
  (`driftwatchdog=gate`, `cargo-*=quality`,
  `sisyphusfy|ariadex|mnemora=agent`,
  `platform-contracts=contract`, `labrys|openpanel|jenkins-local=delivery`,
  `argoscope|devloom=metadata+analytics`) consulting
  `kits/manifest.json` `plugins` array first, `providers.yaml` descriptor
  overriding; `gate/quality/agent/contract_plugins()` beside
  `metadata_plugins()`.
- **Cap grouping CLI.** `forge cap list|inspect <cap>|add|run`
  (`src/cli/cap.rs`, `CapCommands`, `Commands::Cap`): `list` from
  `CAPABILITIES` + live `plugins list` states; `inspect` names Ready
  plugins + mapped flat commands (gate: gate/check/doctor/readiness;
  agent: agent/studio/intent; contract: contract/component/standard;
  delivery: delivery/deploy/publish); `add`/`run` print exact snippets.
  All old flat commands stay as working aliases. Portal projects controls
  gain `forge cap list|inspect` + `cap_group` attribute; web projects view
  gains `#cap-filter` + rail `#cap-badge`. Pinned `tests/browser`
  playwright 1.63.0 reused, no new frontend dep; harness + Rust test
  extended for rail step0, cap list, demo URLs.

Evidence:

| Check | Result |
|---|---|
| `cargo test --test portal_ui_contract` | **18 passed / 0 failed** (13 prior + 5 flywheel: idea entry, next-idea loop, cap filter/badge, demo doc, no-dep) |
| `cargo test --test lifecycle_rail_browser` | **1 passed** — `VERIFIED: lifecycle-rail-check ok` (11 notes: 8 prior + idea entry project+step links, cap badge group, cap filter groups) |
| `cargo test --lib plugins::` | **7 passed / 0 failed** (unknown kind/capability still Invalid) |
| `forge cap list --format json` | groups derive from `CAPABILITIES` (gate present); `delivery ready=1` live from repo providers |
| `forge plugins list --format json` | honest live states with builtin `jenkins`/`openpanel` delivery rows |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **95 passed / 0 failed** active; **94 passed / 0 failed** after archive |
| `git diff --check` | clean |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 500` | **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (390/390 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| `openspec archive flywheel-plugin-cap --yes` | archived as `2026-10-09-flywheel-plugin-cap`, no `--skip-specs`; canonical `portal-web-ui` +4, `forge-publish-plugin-orchestration` +4; `openspec list` leaves only `github-gh-fallback-register` active |

Demo URLs (hookit candidate, shapes verified by harness):

- `/workbench?project=hookit&step=idea` — idea entry (graduation + studio).
- `/workbench?project=hookit&step=spec` — studio spec entry link target.
- `/workbench?project=hookit&step=scaffold` — scaffold.
- `/workbench?project=hookit&step=test` — gate.
- `/workbench?project=hookit&step=deploy` — publish.
- `/workbench?project=hookit&step=operate` — maintain refresh.

Commits on `main`: implementation+specs+UI+playwright `c71805c` (commit
1) + this handoff (commit 2); nothing pushed; `current_spec`
advances back to `github-gh-fallback-register` (active, unarchived).

### lifecycle-series-rail delivered and archived (2026-10-09)

`lifecycle-series-rail` is implemented, verified and archived as
`openspec/changes/archive/2026-10-09-lifecycle-series-rail`,
promoting the `portal-web-ui` delta (+6 requirements, no
`--skip-specs`): part 1's series rail (8 derived steps, single
`aria-current`, `?project=`+`?step=` links), single next-best-action
card and Copy-as-CLI on every confirm action, plus part 2's
observability shortcuts, mobile/a11y hardening and live browser
oracle. `frontend/` only; no API/CLI/portal/catalog change; no new
frontend dependency (pinned `tests/browser` playwright 1.63.0 reused).

Part 2 (this session, sequentially after the part-1 dirty tree):

- **Observability.** Failed doctor/status rows carry a
  `Plan remediate` button previewing the exact
  `forge remediate plan --finding <id>` string (terminal-bound:
  remediate.plan has no web row, `--target` takes a directory the
  browser never sends, so it is omitted and the copy note says to
  run from the project directory); stale/failed/conflict fleet rows
  append a `Refresh & reconcile` deep link to
  `/management?project=`; a recorded-but-unfinished Hermora verb
  renders an inline `Retry Hermora` control opening the existing
  `delivery.hermora-retry` card plus its exact CLI.
- **Coexistence fix (oracle-found).** Workspace discovery re-ran the
  management registered-id redirect on every view, `replaceState`-ing
  workbench deep links to `?project=` alone ~2.5s after load. The
  redirect now fires only on the management view and targets the
  step-preserving `workbenchUrl`.
- **Mobile/a11y.** 232px sidebar collapses to a topbar row under
  860px; `thead th` sticky inside scroll regions; rail `ol` roving
  focus (one Tab stop, arrows/Home/End, per-row
  `<Label>: <done|current step|upcoming>` text alternative);
  `.panel-heading` wraps (fixed a measured 425px page overflow at
  390px); error-summary focus on all six forms pinned by contract.
- **Oracle.** `tests/browser/lifecycle-rail-check.mjs` driven by
  `tests/lifecycle_rail_browser.rs` (exit 2 → UNVERIFIED). Two
  harness defects fixed en route: a `fg.b * fg.b` contrast-composite
  typo (forged ratios) and the panel-tools overflow above.

Evidence:

| Check | Result |
|---|---|
| `cargo test --test portal_ui_contract` | **13 passed / 0 failed** (7 part-1 + 6 part-2) |
| `cargo test --test lifecycle_rail_browser` | **1 passed** — `VERIFIED: lifecycle-rail-check ok` (8 notes: 8 steps/one current/one Tab stop/one Next; step coexistence load+reload; click preserves project + bogus ignored; roving arrows/Home/End; clipboard-equals-shown `forge feature add …`; contrast AA; 390px no overflow) |
| `cargo test --test forge_web_command_catalog_contract` | **9 passed / 0 failed** (no existing token broken) |
| `cargo test --test portal_browser_a11y` | **no target (pre-existing)**: deleted by archived `remove-api-ui`; the part-2 oracle is `lifecycle_rail_browser` above |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `node scripts/check-spec-governance.mjs` | PASS after pointer advance (below) |
| `openspec validate --all --strict --no-interactive` | **94 passed / 0 failed** after archive |
| `git diff --check` + `node --check` (app.js, rail harness) + `cargo fmt --check` | clean |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 500` | **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (389/389 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| `openspec archive lifecycle-series-rail --yes` | archived as `2026-10-09-lifecycle-series-rail`, no `--skip-specs`; canonical `portal-web-ui` +6 (3 part-1 promoted by the archiver + 3 part-2 appended after repairing a dropped delta H1); `openspec list` leaves only `github-gh-fallback-register` active |

Archive repair note: the part-2 delta edit replaced the delta
file's `# portal-web-ui (delta)` H1, so the archiver promoted only
the 3 part-1 requirements under the surviving `## ADDED` header.
The 3 part-2 requirements were appended to the canonical spec
directly (identical wording) and the archived delta's header
restored; validate is green.

Demo URLs (throwaway listeners in the Rust test; shapes, not live):

- `/workbench?project=<id>` — rail + single Next boot.
- `/workbench?project=<id>&step=test` — step hint coexists, survives
  reload, scrolls to the mapped card without moving `aria-current`.
- Rail step links `/workbench?project=<id>&step=<key>` preserve the
  project; `&step=bogus` is ignored.

Commits on `main`: implementation+specs+tests+UI+playwright (commit
1 below) + this handoff (commit 2); nothing pushed; `current_spec`
advances back to `github-gh-fallback-register` (active, unarchived).

### github-gh-fallback-register delivered and archived (2026-10-09)

`github-gh-fallback-register` closes the two 2026-10-09 gaps on `main`.
A prior session implemented and committed it (`216abe1`) and set the
HANDOFF pointer (`ec512c4`) but never ran `openspec archive`, leaving it
active; this closeout archives it, promoting `github-cli-project-workflows`
+1, `github-project-metadata-adapter` +2 and `portal-web-ui` +1, and repairs
the `web-lifecycle-execution` canonical spec's archiver `TBD` Purpose that the
gate flagged as `SPEC_PURPOSE_PLACEHOLDER`:

- **Gap1 — gh-backed fallback for observe/propose.** New
  `src/github/gh_fallback.rs` serves read-only observe via
  `gh repo view <owner/repo> --json ...` and direct single-`topic=`
  propose via `gh repo edit --add-topic` when no
  `forge-github-metadata-adapter`/`FORGE_GITHUB_BIN` binary is
  configured and `gh` is available. Adapter-first ordering;
  `adapter_source=gh-cli-fallback` envelope; `FORGE_GITHUB_TOKEN`
  bypassed only on the fallback path; PR mode and non-topic fields
  still require the adapter. Tokens/credentials never logged
  (fixed notes + `redact_credentials`; live demo below shows no
  secret in output).
- **Gap2 — `create --register-if-missing`.** New flag on
  `GithubCommands::Create` (`src/cli/commands_ops.rs`,
  `src/cli/github.rs`): validates `forge.yaml` via
  `Manifest::load_from_dir`, registers the canonical path via
  `Registry::register`, then runs the existing `gh repo create`
  (+ optional `--push-source`); JSON/human gain
  `register_if_missing`/`registered`. Default path unchanged.
- **Portal + web views.** `repositories` portal controls name the
  three github commands (+ `github_repository`/`github_observe`
  attributes); Delivery gains one `GitHub metadata` card
  (`frontend/index.html`/`app.js`/`styles.css` appended block):
  topics `<ul>` with dev-highlight + text alternative, propose form
  with mode radios + masked confirm-token field, create flow with
  visibility radios + push-source/register-if-missing checkboxes.
  Native controls, labels, error-summary focus, `role="status"`
  results, keyboard operable, no new endpoint/framework.

Live demo (this checkout, `gh` authenticated, no adapter binary):

- `forge project github observe octocat/Hello-World --format json` →
  `adapter_source=gh-cli-fallback (PATH:gh)`,
  `state=current`, real description `My first repository on GitHub!`.
- `... propose <repo> --mode direct --set topic=forge-dev` (no
  `--confirm`) → `error[github-invalid]` (refused, no write).
- `... propose <repo> --set topic=forge-dev` (default PR mode) →
  `error[github-adapter-unavailable]` (PR unchanged, no write).
- `forge project github create --help` names `--register-if-missing`;
  empty-dir + flag → typed invalid refusing before any `gh` write.
- Portal: `forge portal view repositories <id>` lists the three
  github commands. Web: Delivery → GitHub metadata card renders
  topics, CLI previews, focus-moved error summaries.

Evidence:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean (via `cargo fmt`) |
| `cargo build` | 0 errors; 3 pre-existing warnings only (`ShareSurface` unused import, `FleetEntryOutcome::Published`, `FLEET_DEFAULT_JOBS`) |
| `cargo test --test github_gh_fallback_register_contract` | **8 passed / 0 failed** |
| `cargo test --lib github` | **47 passed / 0 failed** |
| `cargo test --bin forge` | **7 passed / 0 failed** |
| `cargo test --test portal_contract` + `--test forge_web_navigation_contract` | **10 + 9 passed / 0 failed** |
| `cargo test` (full workspace) | not bounded locally: exceeds 10min (native-toolchain + `project-runtime` 60s adapter cold-compile); targeted suites above green, no new failure |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **94 passed / 0 failed** |
| `git diff --check` | clean |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate . --timeout-secs 500` | **blocked/unresolved, 0 attributable**: pass — build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (389/389 ≤1000, +1 `gh_fallback.rs`); unresolved pre-existing — declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| `openspec validate --all --strict --no-interactive` (closeout) | **95 passed / 0 failed**; after archive, `openspec list` reports no active changes |
| `git diff --check` (closeout) | clean |
| `forge gate . --timeout-secs 600` (closeout) | **blocked/unresolved, 0 attributable**: pass — build, governance-quality (after the `web-lifecycle-execution` Purpose repair), placeholder-threshold, product-code-boundary, repository, security, source-file-size (396/396 ≤1000); unresolved pre-existing — declared-verification + tests (`project-runtime` 60s cold-compile timeouts) |
| `openspec archive github-gh-fallback-register --yes` | archived as `2026-10-09-github-gh-fallback-register`, no `--skip-specs`; `github-cli-project-workflows` +1, `github-project-metadata-adapter` +2, `portal-web-ui` +1 |
| commits on `main` | implementation `216abe1`; closeout archive + docs this handoff; nothing pushed; no `current_spec` remains |

### portal-feedback-session-recovery delivered and archived (2026-10-09)

`portal-feedback-session-recovery` is implemented, verified and archived as
`openspec/changes/archive/2026-10-09-portal-feedback-session-recovery`,
promoting four `portal-web-ui` requirements (+4, no `--skip-specs`). Closeout
of the standing UI/UX audit's remaining measured gaps on the standalone
`frontend/` assets:

- **G1 — action results are live regions.** The seven `.wb-plan-result`
  containers carry `role="status"`; one `setResultRole(box, isError)` helper
  sets `role`/`aria-live` (`status`/`polite` vs `alert`/`assertive`) on every
  write across workbench plan/apply, delivery action/lookup, workspace-bulk
  preview/run, scoped-management preview/run and each catalog-action card, so
  a reused box never keeps stale urgency.
- **G2 — mid-session `401` recovery.** `request`/`requestStatus` jump to
  `login.html?next=<current path+query>` on a `401` from a dashboard page; the
  login page is excluded, so a wrong-password `401` still renders its inline
  error and summary.
- **G3 — 12px essential-text floor.** One appended slice-7 CSS block raises
  error, hint, digest, CLI-hint, evidence-chip, findings, workflow-reason,
  source-metadata and detail-row text to `--text-md` (12px); uppercase
  micro-labels and badges stay compact.
- **G4 — disclosure association.** Each `.wb-action-body` gets a unique id and
  its `.wb-action-head` sets `aria-controls`.

No API/registry/journal/CLI/catalog change; server `401` semantics, the static
allowlist and the `next` allowlist are untouched.

Evidence:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | 0 errors; the 3 warnings (`ShareSurface` unused import, `FleetEntryOutcome::Published`, `FLEET_DEFAULT_JOBS`) are pre-existing (identical at HEAD `44c3254`) |
| `cargo test --test portal_feedback_session_contract` | **5 passed / 0 failed** |
| frontend contract regression (`--test forge_web_*`, `web_login_credentials_contract`, new test) | static + server contracts green (all `forge_web_*_contract`, `web_login_credentials_contract`, `portal_feedback_session_contract`); browser oracles unchanged |
| browser oracles (`forge_web_project_delivery_browser`, `forge_web_workbench_deep_link_browser`, `forge_web_workspace_onboarding_browser`) | **3 pre-existing failures, reproduced identically against the stashed pre-change `frontend/`** — not attributable to this change: a Playwright actionability/visibility timeout on `#workbench-project`, a Back-URL assertion and a dashboard-order assertion |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **93 passed / 0 failed** |
| `git diff --check` | clean |
| `openspec archive portal-feedback-session-recovery --yes` | archived as `2026-10-09-portal-feedback-session-recovery`, no `--skip-specs`; `openspec list` reports no active changes |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate . --timeout-secs 600` | **blocked/unresolved, 0 attributable**: pass — build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (388/388 ≤1000); unresolved pre-existing — declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget cold-compiling TLS deps `ring`/`rustls`, the same pre-existing pair recorded by `automated-size-split-batch`). Remediation: warm/point the adapter at the shared target cache or raise the workspace-governance rule-pack per-check budget, then re-run `forge gate .`. This frontend-only change adds no new failure. |

### automated-size-split-batch delivered and archived (2026-10-09)

`automated-size-split-batch` is implemented, verified and archived as
`openspec/changes/archive/2026-10-09-automated-size-split-batch`,
promoting the `automated-size-split-batch` spec (+2 requirements) without
`--skip-specs`. Every `src/**/*.rs` is now under the 1000-line cap
(46 oversized → 0; 5 largest now 997/996/975/970/959). Method: splitrs
(`--max-lines 900 --naming-strategy domain-specific`, one file at a time,
`--rollback`) + readability rename of generic buckets to domain names +
manual sub-splits where a bucket exceeded 900. Bodies verbatim
(item-count parity per file); public paths stable via re-exports; no
caller edits; no API/CLI/JSON behavior change; no frontend change.

Division of labor (operator direction): subagents did splits with
lightweight checks only (fmt, constrained build, module lib-tests); the
orchestrator ran the heavy full suite + Gate once, fixed 2 forced test
path-scan updates + 2 split-attributable import warnings, and closed out.

Evidence:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build -j2` (sccache-warmed) | 0 errors; remaining warnings verified pre-existing via HEAD (validation ShareSurface, fleet_data dead-code, cli dead-code, FLEET_DEFAULT_JOBS) |
| lib `--lib` | **1209 passed / 0 failed / 1 ignored** |
| bin `--bin forge` | **7 / 7 passed** |
| integration targets (full coverage, 130+) | all green except 7 pre-existing failures (below); fixed by this change: catalog_contract 9/9, workspace_metadata 4/4 |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `node scripts/check-spec-governance.mjs` | PASS (after TBD + pointer fix) |
| `openspec validate --all --strict --no-interactive` | **93 passed / 0 failed** |
| `git diff --check` | clean |
| `openspec archive automated-size-split-batch --yes` | archived as `2026-10-09-automated-size-split-batch`, no `--skip-specs`; `openspec list` reports no active changes |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **source-file-size PASS (388/388 ≤1000)**; pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; unresolved pre-existing: declared-verification + tests (`project-runtime` 60s adapter timeouts); 0 attributable |

Process note: the gate size check consumes the git index, so its first
post-split run errored on uncommitted deletions; implementation was
committed as `f853d11` before the passing run (order deviation).

Pre-existing failures unrelated to this change (recorded, not fixed):

- `forge_web_project_delivery_browser` selectOption on invisible
  `#workbench-project` (recorded earlier; harness must navigate to
  `/workbench` first).
- `forge_web_project_status_contract` + `forge_web_project_workbench_contract`
  `innerHTML` assertions (introduced by archived slice 5 `0cee5ad`; this
  change touches 0 frontend files). Remediation: replace the single
  `innerHTML` use with safe DOM construction in a follow-up frontend change.
- `forge_web_workbench_deep_link_browser` back/forward race (flaky both
  trees: batch 2/6 vs pristine-stash 2/4, identical signature; timing-sensitive
  history assertions). Remediation: harden harness waits in a follow-up change.
- `forge_web_workspace_onboarding_browser` dashboard-order assertion
  (recorded earlier; `#commands-title` gone since lifecycle-tracked view).
- `portfolio_ui_contract` 20 failures (asserts `/ui/` routes deleted by
  archived remove-api-ui; zero batch touches, zero routes at committed HEAD).
  Remediation: delete or rewrite the stale file against the JSON API.
- `project_query_surface_contract` 2 failures (403-vs-401: no-bearer catalog
  requests hit the maintainer-platform cookie-path origin check; logic
  byte-identical at committed HEAD). Remediation: send the configured origin
  in the test (expect 401) or assert both branches.

Implementation commit: `f853d11`. Nothing pushed. No `current_spec`
pointer remains because no OpenSpec change is active.

### portfolio-mod-size-split delivered and archived (2026-10-08)

File 5 of the source-file-size grind per
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9 (verbatim-move rules per §10 decision ledger). Second `src/`-side
file (file 4 was `src/portfolio/interest/`); no behavior change
anywhere.

Move (`src/portfolio/mod.rs` 1019 → 799 lines, new
`src/portfolio/vocabulary.rs` 228 lines): the four closed
vocabularies (`Lifecycle`, `Confidence`, `RelationType`,
`EvidenceStatus` + impls) copied verbatim — verified by diffing
the extracted block against `HEAD:src/portfolio/mod.rs`
(0 diff over 220 lines, still 0 diff after `cargo fmt`).
`mod.rs` gains `pub mod vocabulary;` +
`pub use vocabulary::{Confidence, EvidenceStatus, Lifecycle,
RelationType}`, so every `crate::portfolio::<name>` path
resolves as before; callers in `src/registry/portfolio.rs`,
`src/api/portfolio.rs`, `src/api/mod.rs`, `src/main.rs` resolve
unchanged. No other `src/` file touched; no test logic change.

Evidence at archive (change active for verification, archived after):

| Check | Result |
|---|---|
| pre-move baseline `cargo build` | 0 errors; 2 pre-existing warnings |
| pre-move `cargo test --lib portfolio::` | **106 passed / 0 failed** |
| pre-move `cargo test --test portfolio_contract` | **23 passed / 0 failed** |
| post-move `cargo test --lib portfolio::` | **106 passed / 0 failed** (identical) |
| post-move `cargo test --test portfolio_contract` | **23 passed / 0 failed** (identical) |
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` post-move | 0 errors; same 2 pre-existing warnings |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `node scripts/check-spec-governance.mjs` | **PASS** after archive (archiver-stamped TBD Purpose replaced with a source-backed sentence; stale pointer removed) |
| `openspec validate --all --strict --no-interactive` | **92 passed / 0 failed** active; **92 passed / 0 failed** after archive (+1 new canonical spec) |
| `git diff --check` | clean |
| `./target/debug/forge gate --dry-run` | plan rendered; 9 required checks |
| `./target/debug/forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **46 of 121** (pre-existing; was 47 of 120 — oversized count down exactly 1, denominator +1 for the new file); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec archive portfolio-mod-size-split --yes` | `portfolio-mod-size-split: create` (+1); **no `--skip-specs`**; archived as `2026-10-08-portfolio-mod-size-split`; `openspec list` reports **no active changes** |

Pre-existing failures not attributable to this change (recorded, not
fixed — separate harnesses, separate changes): `source-file-size`
(46/121 after vs 47/120 before — this change moves exactly one file
under the cap; `interest/activation.rs` 1054 and
`registry/interest/mod.rs` 1047 remain for their own future changes)
and the two `project-runtime` adapter-timeout unresolved items;
remediation paths unchanged from prior entries (src-file splitting
program; harness timeout investigation).

Files changed: `src/portfolio/mod.rs` (1019→799),
`src/portfolio/vocabulary.rs` (new, 228), new
`openspec/changes/archive/2026-10-08-portfolio-mod-size-split/`
(proposal, design, tasks, delta spec), new canonical spec
`openspec/specs/portfolio-mod-size-split/spec.md` (+1),
`HANDOFF.md` (this entry).

Commits: implementation+archive+spec-promotion (this commit), HANDOFF
evidence (this commit). `openspec list` reports no active changes; no
`current_spec` pointer remains. Nothing pushed.

### portfolio-interest-size-split delivered and archived (2026-10-08)

File 4 of the source-file-size grind per
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9 (verbatim-move rules per §10 decision ledger). First `src/`-side
file (prior three were tests-side); no behavior change anywhere.

Move (`src/portfolio/interest/mod.rs` 1002 → 778 lines, new
`src/portfolio/interest/vocabulary.rs` 238 lines): the four closed
vocabularies (`PrivacyMode`, `Coverage`, `InterestMetric`,
`SnapshotState` + impls) copied verbatim — verified by diffing the
extracted block against `HEAD:src/portfolio/interest/mod.rs`
(0 diff over 228 lines). `mod.rs` gains `pub mod vocabulary;` +
`pub use vocabulary::{Coverage, InterestMetric, PrivacyMode,
SnapshotState}`, so every `crate::portfolio::interest::<name>`
path resolves as before; module doc seam updated. No other `src/`
file touched; no test logic change.

Evidence at archive (change active for verification, archived after):

| Check | Result |
|---|---|
| pre-move baseline (stashed pristine) `cargo build` | 0 errors; 2 pre-existing warnings |
| pre-move `cargo test --lib portfolio::interest` | **44 passed / 0 failed** |
| pre-move interest/activation contract suites | interest_api **13**, interest_cli **16**, interest_cross **10**, activation_cli **16**, activation_cross **6**, portfolio_contract **23** — all passed, 0 failed |
| post-move `cargo test --lib portfolio::interest` | **44 passed / 0 failed** (identical) |
| post-move contract suites | **13 / 16 / 10 / 16 / 6 / 23 passed, 0 failed** (identical) |
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` post-move | 0 errors; same 2 pre-existing warnings |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `node scripts/check-spec-governance.mjs` | **PASS** after archive (archiver-stamped TBD Purpose replaced with a source-backed sentence; stale pointer removed) |
| `openspec validate --all --strict --no-interactive` | **91 passed / 0 failed** active; **91 passed / 0 failed** after archive (+1 new canonical spec) |
| `git diff --check` | clean |
| `./target/debug/forge gate --dry-run` | plan rendered; 9 required checks |
| `./target/debug/forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **47 of 120** (pre-existing; was 48 of 119 — oversized count down exactly 1, denominator +1 for the new file); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec archive portfolio-interest-size-split --yes` | `portfolio-interest-size-split: create` (+1); **no `--skip-specs`**; archived as `2026-10-08-portfolio-interest-size-split`; `openspec list` reports **no active changes** |

Pre-existing failures not attributable to this change (recorded, not
fixed — separate harnesses, separate changes): `source-file-size`
(47/120 after vs 48/119 before — this change moves exactly one file
under the cap; `activation.rs` 1054, `registry/interest/mod.rs`
1047, `portfolio/mod.rs` 1019 remain for their own future changes)
and the two `project-runtime` adapter-timeout unresolved items;
remediation paths unchanged from prior entries (src-file splitting
program; harness timeout investigation).

Files changed: `src/portfolio/interest/mod.rs` (1002→778),
`src/portfolio/interest/vocabulary.rs` (new, 238), new
`openspec/changes/archive/2026-10-08-portfolio-interest-size-split/`
(proposal, design, tasks, delta spec), new canonical spec
`openspec/specs/portfolio-interest-size-split/spec.md` (+1),
`HANDOFF.md` (this entry).

Commits: implementation+archive+spec-promotion `53735dc`, HANDOFF
evidence (this commit). `openspec list` reports no active changes; no
`current_spec` pointer remains. Nothing pushed.

### portfolio-contract-size-split delivered and archived (2026-10-08)

File 3 of the source-file-size grind per
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9 (verbatim-move rules per §10 decision ledger). Tests-side only;
no behavior change anywhere.

Move (`tests/portfolio_contract.rs` 1261 lines deleted,
`tests/portfolio_contract/` created): `main.rs` (file doc, all
shared helpers, `drive` / `api_request` / `api_json` /
`seed_two_projects` seed fixtures, two `mod` declarations; `fn` →
`pub(crate) fn` per the `tests/kit_contract/` precedent) plus two
CLI-vs-HTTP submodules — `cli` (17), `http` (6). Every one of the
23 `#[test]` function bodies copied verbatim (verified by
substring assertion against the original at split time: each of
the 23 names occurs exactly once across the three new files); no
cleanup, no renames. No `src/` change, no `mod` change in
`src/lib.rs` or `src/main.rs`, no public symbol/route/CLI/env
change.

Evidence at archive (change active for verification, archived after):

| Check | Result |
|---|---|
| `cargo test --test portfolio_contract` pre-move | **23 passed / 0 failed / 0 ignored** |
| `cargo test --test portfolio_contract` post-move | **23 passed / 0 failed / 0 ignored** (identical) |
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; 2 pre-existing warnings only |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `node scripts/check-spec-governance.mjs` | **PASS** after archive (archiver-stamped TBD Purpose replaced with a source-backed sentence; stale pointer removed) |
| `openspec validate --all --strict --no-interactive` | **90 passed / 0 failed** active; **90 passed / 0 failed** after archive (+1 new canonical spec) |
| `git diff --check` | clean |
| `./target/debug/forge gate --dry-run` | plan rendered; 9 required checks |
| `./target/debug/forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change is tests-side only and the gate evaluates `src/`, so the count is unchanged); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec archive portfolio-contract-size-split --yes` | `portfolio-contract-size-split: create` (+1); **no `--skip-specs`**; archived as `2026-10-08-portfolio-contract-size-split`; `openspec list` reports **no active changes** |

Pre-existing failures not attributable to this change (recorded, not
fixed — separate harnesses, separate changes): `source-file-size`
(`src/`-only gate; 48/119 both before and after) and the two
`project-runtime` adapter-timeout unresolved items; remediation paths
unchanged from prior entries (src-file splitting program; harness
timeout investigation).

The delta spec carried a real `## Purpose` sentence, but the
archiver still stamped TBD; the canonical spec's Purpose was
replaced with the source-backed sentence above in this same change
(governance PASS after).

Files changed: `tests/portfolio_contract.rs` (deleted),
`tests/portfolio_contract/` (`main.rs` + 2 submodules), new
`openspec/changes/archive/2026-10-08-portfolio-contract-size-split/`
(proposal, design, tasks, delta spec), new canonical spec
`openspec/specs/portfolio-contract-size-split/spec.md` (+1),
`HANDOFF.md` (this entry).

Commits: implementation+archive+spec-promotion `e007840`, HANDOFF
evidence (this commit). `openspec list` reports no active changes; no
`current_spec` pointer remains. Nothing pushed.

### supervised-agent-contract-size-split delivered and archived (2026-10-08)

File 2 of the source-file-size grind per
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9 (verbatim-move rules per §10 decision ledger). Tests-side only;
no behavior change anywhere.

Move (`tests/supervised_agent_contract.rs` 1201 lines deleted,
`tests/supervised_agent_contract/` created): `main.rs` (file doc,
all shared helpers, `Fixture` struct + impl, `code_of`,
four `mod` declarations; `fn` → `pub(crate) fn` per the
`tests/kit_contract/` precedent) plus four provider-scenario
submodules — `legacy_session` (3), `ariadex` (12),
`sisyphusfy` (9), `native_toolchain` (5). Every one of the 29
`#[test]` function bodies copied verbatim (verified by substring
assertion against the original at split time); no cleanup, no
renames. No `src/` change, no `mod` change in `src/lib.rs` or
`src/main.rs`, no public symbol/route/CLI/env change.

Evidence at archive (change active for verification, archived after):

| Check | Result |
|---|---|
| `cargo test --test supervised_agent_contract` pre-move | **29 passed / 0 failed / 0 ignored** |
| `cargo test --test supervised_agent_contract` post-move | **29 passed / 0 failed / 0 ignored** (identical) |
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; pre-existing warnings only |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `node scripts/check-spec-governance.mjs` | **PASS** after archive (archiver-stamped TBD Purpose replaced with a source-backed sentence; stale pointer removed) |
| `openspec validate --all --strict --no-interactive` | **89 passed / 0 failed** active; **89 passed / 0 failed** after archive (+1 new canonical spec) |
| `git diff --check` | clean |
| `./target/debug/forge gate --dry-run` | plan rendered; 9 required checks |
| `./target/debug/forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change is tests-side only and the gate evaluates `src/`, so the count is unchanged); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec archive supervised-agent-contract-size-split --yes` | `supervised-agent-contract-size-split: create` (+1); **no `--skip-specs`**; archived as `2026-10-08-supervised-agent-contract-size-split`; `openspec list` reports **no active changes** |

Pre-existing failures not attributable to this change (recorded, not
fixed — separate harnesses, separate changes): `source-file-size`
(`src/`-only gate; 48/119 both before and after) and the two
`project-runtime` adapter-timeout unresolved items; remediation paths
unchanged from prior entries (src-file splitting program; harness
timeout investigation).

One implementation note: the split script's visibility regex briefly
over-matched a `proj: &Path` function parameter (`pub(crate) proj`);
reverted to the verbatim parameter before `cargo fmt`, so the
shipped helpers differ from the original only by `pub(crate)` on
the items themselves. The delta spec carried a real `## Purpose`
sentence, but the archiver still stamped TBD; the canonical spec's
Purpose was replaced with the source-backed sentence above in this
same change (governance PASS after).

Files changed: `tests/supervised_agent_contract.rs` (deleted),
`tests/supervised_agent_contract/` (`main.rs` + 4 submodules), new
`openspec/changes/archive/2026-10-08-supervised-agent-contract-size-split/`
(proposal, design, tasks, delta spec), new canonical spec
`openspec/specs/supervised-agent-contract-size-split/spec.md` (+1),
`HANDOFF.md` (this entry).

Commits: implementation+archive+spec-promotion `7b6cc1b`, HANDOFF
evidence (this commit). `openspec list` reports no active changes; no
`current_spec` pointer remains. Nothing pushed.

### gate-contract-size-split delivered and archived (2026-10-08)

File 1 of the source-file-size grind per
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9 (verbatim-move rules per §10 decision ledger). Tests-side only;
no behavior change anywhere.

Move (`tests/gate_contract.rs` 1157 lines deleted,
`tests/gate_contract/` created): `main.rs` (file doc, all shared
helpers, `GateRun` struct, five `mod` declarations; `fn` →
`pub(crate) fn` per the `tests/kit_contract/` precedent) plus five
scenario-family submodules — `documented_help` (5),
`passing` (7), `blocked` (7), `review_required` (1),
`unknown_runtime` (8). Every one of the 28 `#[test]` function
bodies copied verbatim (verified by substring assertion against
the original); no cleanup, no renames. No `src/` change, no `mod`
change in `src/lib.rs` or `src/main.rs`, no public
symbol/route/CLI/env change.

Evidence at archive (change active for verification, archived after):

| Check | Result |
|---|---|
| `cargo test --test gate_contract` pre-move | **28 passed / 0 failed / 0 ignored** |
| `cargo test --test gate_contract` post-move | **28 passed / 0 failed / 0 ignored** (identical) |
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; 2 pre-existing warnings only |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **88 passed / 0 failed** active; **88 passed / 0 failed** after archive (+1 new canonical spec) |
| `git diff --check` | clean |
| `./target/debug/forge gate --dry-run` | plan rendered; 9 required checks |
| `./target/debug/forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change is tests-side only and the gate evaluates `src/`, so the count is unchanged); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec archive gate-contract-size-split --yes` | `gate-contract-size-split: create` (+1); **no `--skip-specs`**; archived as `2026-10-08-gate-contract-size-split`; `openspec list` reports **no active changes** |

Pre-existing failures not attributable to this change (recorded, not
fixed — separate harnesses, separate changes): `source-file-size`
(`src/`-only gate; 48/119 both before and after) and the two
`project-runtime` adapter-timeout unresolved items; remediation paths
unchanged from prior entries (src-file splitting program; harness
timeout investigation).

Files changed: `tests/gate_contract.rs` (deleted),
`tests/gate_contract/` (`main.rs` + 5 submodules), new
`openspec/changes/archive/2026-10-08-gate-contract-size-split/`
(proposal, design, tasks, delta spec), new canonical spec
`openspec/specs/gate-contract-size-split/spec.md` (+1),
`HANDOFF.md` (this entry).

Commits: implementation+archive+spec-promotion `15081e8`, HANDOFF
evidence (this commit). `openspec list` reports no active changes; no
`current_spec` pointer remains. Nothing pushed.

### portal-data-table-performance delivered and archived (2026-10-08)

Slice 6 of the frontend UI/UX audit (LAST; queue is now empty).
Sortable fleet columns with announced sort state, browser-side CSV
export of the filtered rows, windowed/paginated rendering with honest
counts, skeleton/shimmer placeholders with reserved count space.
`frontend/` only, vanilla HTML/CSS/JS; no API/registry/journal/CLI/
catalog change — the catalog `limit:1000` query and JSON shape stay
exactly as-is.

Fix (`frontend/index.html` sort headers + export button + pager,
`frontend/app.js` filter→sort→page pipeline + CSV + skeleton,
`frontend/styles.css` appended slice-6 block,
`tests/forge_web_navigation_contract.rs` +1 test):

- Sort: fleet `Project`/`Details`/`Status` headers become native
  `<button>` controls (`Sort by <column>`, 44px minima) with
  `aria-sort` on the `<th>` (`none` default, `ascending`/`descending`
  on the active column); click cycles asc→desc→none (none restores API
  order). Keys reuse the exact rendered strings (`name || identity`,
  `fleetDetailsLine`, `projectStatusLine`, lowercased); ties break by
  pre-sort index (stable). Sort applies after the untouched
  free-text/source/catalog-predicate filter.
- Export: `#fleet-export` (`Export filtered CSV`, disabled when the
  filtered set is empty) downloads the filtered+sorted (pre-page) rows
  as RFC-4180-escaped CSV (`name,identity,profile,details,status,
  source,management`) via `Blob` + temp anchor
  (`forge-projects.csv`); no endpoint, no server state.
- Paging: `FLEET_PAGE_SIZE = 50` — at most one page mounted;
  `#project-count` reports the honest window (`Showing X of Y
  filtered (Z total)`); `#fleet-pager` Prev/Next (44px, labelled)
  plus `#fleet-page-info` (`Page N of M`, `aria-live="polite"`).
  Filter/sort changes reset to page 1 (clamped on shrink).
- Loading: `renderFleetSkeleton` mounts 8 shimmer rows with
  `aria-busy="true"` while `/v1/admin/projects` is in flight (error
  path clears them via `showDashboardError`); delivery/maintain
  loaders use the same `.skeleton` span (sr-only text kept for AT);
  summary counts reserve space (`min-height`, `min-width:3ch`);
  shimmer is background-position-only and collapses under the
  `prefers-reduced-motion` guard; no layout property animates.

Evidence at archive (change active for verification, archived after):

| Check | Result |
|---|---|
| `node --check frontend/app.js` + HTML parse | clean |
| static token audit over shipped files | **all present**: sort/export/pager/skeleton tokens; `limit:1000` query intact; prior-slice tokens intact (`lastRouteView`, filter-state, `view-unknown`, `--z-skip:60`, `--icon-stroke:1.8`, `--dur-enter:140ms`, 44px, touch, summaries); bare `120ms ease` still absent; no new export endpoint |
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; 2 pre-existing warnings only |
| `forge_web_navigation_contract` | **9 passed / 0 failed** (+1 slice-6 token test) |
| `forge_web_manage_deep_link_browser` (real Chromium) | **1 passed / 0 failed** (no regression; exercises the refactored render path) |
| `forge_web_workbench_deep_link_browser` (real Chromium) | **1 passed / 0 failed** (no regression) |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **88 passed / 0 failed** active; **87 passed / 0 failed** after archive |
| `git diff --check` | clean |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change adds no `src/` file); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec archive portal-data-table-performance --yes` | `portal-web-ui: update` (+4); **no `--skip-specs`**; archived as `2026-10-08-portal-data-table-performance`; `openspec list` reports **no active changes** |

Pre-existing failures not attributable to this change (recorded, not
fixed — separate harnesses, separate changes): `source-file-size`
(`src/`-only gate; 48/119 both before and after) and the two
`project-runtime` adapter-timeout unresolved items; remediation paths
unchanged from prior slices (src-file splitting program; harness
timeout investigation).

Files changed: `frontend/app.js`, `frontend/index.html`,
`frontend/styles.css`, `tests/forge_web_navigation_contract.rs`,
`openspec/specs/portal-web-ui/spec.md` (+4), new
`openspec/changes/archive/2026-10-08-portal-data-table-performance/`
(proposal, design, tasks, delta spec), `HANDOFF.md` (this entry).

Commits: implementation+archive+spec-promotion `bbd0773`, HANDOFF
evidence (this commit). `openspec list` reports no active changes; no
`current_spec` pointer remains. Nothing pushed. Queue is empty: all six
UI/UX slices delivered (fleet-manage-deep-link,
portal-focus-route-contrast, portal-touch-responsive-targets,
portal-form-error-feedback, portal-layout-navigation,
portal-icon-type-motion, portal-data-table-performance).

### portal-icon-type-motion delivered and archived (2026-10-08)

Slice 5 of the frontend UI/UX audit (consistent icon set, readable
type scale, shared motion tokens; slice 6 stays QUEUED, not authored).
`frontend/` only, vanilla HTML/CSS/JS; no API/registry/journal/CLI/
catalog change. No external font/CDN fetch — system stacks and inline
SVG only.

Fix (`frontend/index.html`, `frontend/login.html`, `frontend/app.js`,
`frontend/styles.css` appended slice-5 block + 3 surgical base edits,
`tests/forge_web_navigation_contract.rs` +1 test):

- Icons: all 23 glyph sites become inline SVG from one stroke set (24
  viewBox, `currentColor`, round caps/joins, `--icon-stroke:1.8`,
  `--icon-sm/md/lg` 14/16/20px) — 5 sidebar nav, 1 search, 8 summary
  cards, 2 empty marks (`index.html`); 3 story checks (new
  `.check-disc` spans), lock, submit arrow (`login.html`); workbench
  disclosure caret `▸` → chevron SVG and submit restore-string arrow
  (`app.js`, zero logic change — the 90° open-rotation rule keeps
  working on the wrapper span). Decorative SVGs keep `aria-hidden`;
  all visible labels, `sr-only` names, and `aria-current` intact; no
  emoji anywhere (`·`/`—`/prose arrows are punctuation, kept).
- Type: explicit `html{font-size:16px}` base, body `1rem/1.6` (the one
  intended vertical change), `--text-*` scale wired at identical
  computed sizes, `tabular-nums` for counts/ids/digests/timestamps,
  `70ch` guard on previously-uncapped prose (already-narrow blocks
  untouched), `.wb-digest` `break-all` → `overflow-wrap:anywhere`. No
  color token touched — every 4.5:1 pair stands by construction.
- Motion: shared `--dur-enter:140ms` (inside the slice-2 80–150ms
  press band) / `--dur-exit:90ms` (≈64% of enter) with
  `--ease-standard`/`--ease-out`; slice-2 declarations rewired to the
  tokens (single source, no dead literals), press/active states exit
  fast, caret rotates on the enter token; transform/opacity-only, no
  layout-shift animation, reduced-motion guard kept verbatim.

Evidence at archive (change active for verification, archived after):

| Check | Result |
|---|---|
| `node --check frontend/app.js` + HTML parse | clean |
| glyph purge over shipped files | **0 remaining** (`▦⚙＋◈⇪⌕⌁✔↻⌀Σ✓●▸` gone from icon sites; `→` gone from button markup; `break-all` gone) |
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; 2 pre-existing warnings only |
| `forge_web_navigation_contract` | **8 passed / 0 failed** (+1 slice-5 token test) |
| `forge_web_manage_deep_link_browser` (real Chromium) | **1 passed / 0 failed** (no regression) |
| `forge_web_workbench_deep_link_browser` (real Chromium) | **1 passed / 0 failed** (no regression; one initial flake, green on 2 consecutive reruns) |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **88 passed / 0 failed** active; **87 passed / 0 failed** after archive |
| `git diff --check` | clean |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change adds no `src/` file); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec archive portal-icon-type-motion --yes` | `portal-web-ui: update` (+3); **no `--skip-specs`**; archived as `2026-10-08-portal-icon-type-motion`; `openspec list` reports **no active changes** |

Pre-existing failures not attributable to this change (recorded, not
fixed — separate harnesses, separate changes): `source-file-size`
(`src/`-only gate; 48/119 both before and after) and the two
`project-runtime` adapter-timeout unresolved items; remediation paths
unchanged from prior slices (src-file splitting program; harness
timeout investigation).

Files changed: `frontend/app.js`, `frontend/index.html`,
`frontend/login.html`, `frontend/styles.css`,
`tests/forge_web_navigation_contract.rs`,
`openspec/specs/portal-web-ui/spec.md` (+3), new
`openspec/changes/archive/2026-10-08-portal-icon-type-motion/`
(proposal, design, tasks, delta spec), `HANDOFF.md` (this entry).

Commits: implementation+archive+spec-promotion `0cee5ad`, HANDOFF
evidence (this commit). `openspec list` reports no active changes; no
`current_spec` pointer remains. Nothing pushed.

### portal-layout-navigation delivered and archived (2026-10-08)

Slice 4 of the frontend UI/UX audit (responsive layout, nav
reachability, filter state, unknown fallback, z-index layering;
slices 5–6 stay QUEUED, not authored). `frontend/` only, vanilla
HTML/CSS/JS; no API/registry/journal/CLI/catalog change.

Fix (`frontend/index.html` unknown section, `frontend/app.js`
router + filter snapshot, `frontend/styles.css` appended slice-4
override block, `tests/forge_web_navigation_contract.rs` +1 test):

- Breakpoints toward 375/768/1024/1440: new `1024px` content-padding
  tightening, new `375px` small-phone compaction, short-landscape gate
  (`orientation:landscape` + `max-height:500px`, desktop landscape
  excluded) compacting the auth hero; existing `850px` rule is the 768
  tablet step; wide end toward 1440 stays bounded by the
  `min(1260px,100%)` cap. Desktop above 1024px unchanged
  (`styles.css` diff is purely additive — 0 removed lines).
- 375px sidebar: nav becomes an in-row `overflow-x:auto` scroller with
  `flex:none` nowrap links (brand-above/nav-below wrap at 375px); all 5
  destinations reachable with no page-level horizontal scroll; active
  link keeps `.nav-active` + `aria-current`; 44px minima kept.
- Filter/search preservation: 8 controls (search, source, six
  predicates) snapshot to tab-scoped `sessionStorage`
  (`forge.filter-state.v1`, typed strings only) on input/change and on
  `navigateTo`; restored on dashboard boot (before first table render)
  and on switching back into the projects view, re-running the
  predicate catalog query or local render. `?project=` never
  snapshotted/restored — URL stays its source of truth.
- Unknown route: `renderRoute` checks `isRoutePath` first; unknown
  pathnames show the new `#view-unknown` section (explanation + 5 real
  44px destination links), clear all nav active states, set the
  "Page not found" crumb/title, and participate in the `lastRouteView`
  focus guard. Missing section null-guards to the prior default.
- Z-index scale: `--z-sticky:10 / --z-nav:30 / --z-dropdown:40 /
  --z-banner:45 / --z-overlay:50 / --z-skip:60` wired to topbar,
  sidebar, dropdown wrappers, notices/summaries, open action cards,
  and the skip link (topmost; 20→60).

Evidence at archive (change active for verification, archived after):

| Check | Result |
|---|---|
| `node --check frontend/app.js` | clean |
| static token audit over shipped files | **16 / 16**: 1024px + 375px rules, short-landscape gate, sidebar scroll row + 375 wrap, 6-rung z-scale + skip/topbar/sidebar wiring, unknown section + 5 links + branch, 8-control snapshot + helpers, 44px minima, prior deep-link tokens, prior slice tokens, 12-endpoint route set identical before/after |
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; 2 pre-existing warnings only |
| `forge_web_navigation_contract` | **7 passed / 0 failed** (+1 slice-4 token test) |
| `forge_web_manage_deep_link_browser` (real Chromium) | **1 passed / 0 failed** (no regression) |
| `forge_web_workbench_deep_link_browser` (real Chromium) | **1 passed / 0 failed** (no regression) |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **88 passed / 0 failed** active; **87 passed / 0 failed** after archive |
| `git diff --check` | clean |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change adds no `src/` file); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec archive portal-layout-navigation --yes` | `portal-web-ui: update` (+5); **no `--skip-specs`**; archived as `2026-10-08-portal-layout-navigation`; `openspec list` reports **no active changes** |

Pre-existing failures not attributable to this change (recorded, not
fixed — separate harnesses, separate changes): `source-file-size`
(`src/`-only gate; 48/119 both before and after) and the two
`project-runtime` adapter-timeout unresolved items; remediation paths
unchanged from prior slices (src-file splitting program; harness
timeout investigation).

Files changed: `frontend/app.js`, `frontend/index.html`,
`frontend/styles.css`, `tests/forge_web_navigation_contract.rs`,
`openspec/specs/portal-web-ui/spec.md` (+5), new
`openspec/changes/archive/2026-10-08-portal-layout-navigation/`
(proposal, design, tasks, delta spec), `HANDOFF.md` (this entry).

Commits: implementation+archive+spec-promotion `977e10f`, HANDOFF
evidence (this commit). `openspec list` reports no active changes; no
`current_spec` pointer remains. Nothing pushed.

### portal-form-error-feedback delivered and archived (2026-10-08)

Slice 3 of the frontend UI/UX audit (form labels + error feedback;
slices 4–6 stay QUEUED, not authored). `frontend/` only, vanilla
HTML/CSS/JS; no API/registry/journal/CLI/catalog change.

Fix (`frontend/index.html`, `frontend/login.html`, `frontend/app.js`,
`frontend/styles.css` appended slice-3 override block):

- Visible labels: fleet filter row (6 inputs) uses stacked visible
  labels (placeholders kept as examples) + one shared row hint
  (blank-means-any); delivery allowlist/publish/reconcile/lookup inputs
  and portfolio tag/review/project selects relabelled from `sr-only` to
  visible `label.field` stacks with per-field helper text; workbench
  action-card boolean params gain visible labels and all card fields a
  visible `*` required marker. 44px targets and layout from slice 2
  unchanged (`.field` inputs re-assert `min-height:44px`).
- Error summaries: one shared `renderErrorSummary` helper (heading +
  per-field links, focus moved to the `tabindex="-1"` container,
  retained inline errors) plus `setFieldError`/`clearFieldError`
  (`aria-invalid`, `aria-describedby` add/remove) wired into login,
  portfolio add-tag/record-review (validation + server), delivery
  set/approve/publish/reconcile/lookup (validation + refusals),
  `buildActionControl` cards (gather + refused preview/run), and
  workspace bulk / management scoped preview/run. `fleet-filter-error`
  `role="status"` → `role="alert"`.
- Login: password show/hide toggle (`type=button`, `aria-pressed`,
  `aria-controls`, Show/Hide; value/`name`/`autocomplete` untouched, no
  paste blocking), visible `*` required indicators + legend line.
- Typed-field guarantee intact: payload shapes and endpoints unchanged
  (fetch route set identical before/after); no new endpoint.

Evidence at archive (change active for verification, archived after):

| Check | Result |
|---|---|
| `node --check frontend/app.js` | clean |
| static token audit over shipped files | **35 / 35**: 19 scoped visible labels, fleet `role=alert`, 5 summary containers + card summary, toggle + pressed/controls, autocomplete/names intact, required markers, 6 fleet `aria-describedby`, 4 helpers + focus move, fetch routes unchanged, old router/deep-link tokens intact, slice-3 CSS tokens |
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; 2 pre-existing warnings only |
| `forge_web_navigation_contract` | **6 passed / 0 failed** |
| `forge_web_manage_deep_link_browser` (real Chromium) | **1 passed / 0 failed** (no regression) |
| `forge_web_workbench_deep_link_browser` (real Chromium) | **1 passed / 0 failed** (no regression) |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **88 passed / 0 failed** active; **87 passed / 0 failed** after archive |
| `git diff --check` | clean |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change adds no `src/` file); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec archive portal-form-error-feedback --yes` | `portal-web-ui: update` (+3); **no `--skip-specs`**; archived as `2026-10-08-portal-form-error-feedback`; `openspec list` reports **no active changes** |

Pre-existing failures not attributable to this change (recorded, not
fixed — separate harnesses, separate changes): `source-file-size`
(`src/`-only gate; 48/119 both before and after) and the two
`project-runtime` adapter-timeout unresolved items; remediation paths
unchanged from prior slices (src-file splitting program; harness
timeout investigation).

Files changed: `frontend/app.js`, `frontend/index.html`,
`frontend/login.html`, `frontend/styles.css`,
`openspec/specs/portal-web-ui/spec.md` (+3), new
`openspec/changes/archive/2026-10-08-portal-form-error-feedback/`
(proposal, design, tasks, delta spec), `HANDOFF.md` (this entry).

Commits: implementation+archive+spec-promotion `bb0c9b6`, HANDOFF
evidence (this commit). `openspec list` reports no active changes; no
`current_spec` pointer remains. Nothing pushed.

### portal-touch-responsive-targets delivered and archived (2026-10-08)

Slice 2 of the frontend UI/UX audit (touch/manipulation responsiveness;
slices 3–6 stay QUEUED, not authored). `frontend/` only, vanilla
HTML/CSS/JS; no API/registry/journal/CLI/catalog change.

Fix (`frontend/styles.css` appended slice-2 override block,
`frontend/index.html`, `frontend/login.html`, `frontend/app.js`):

- Every operator control computes `min-height >= 44px` via `min-height`
  (padding/font density untouched): `.button` (36→44), `.button-quiet`
  (32→44), `.button-primary` (42→44), `.nav-link` (none→44),
  `.filter-box` (32→44, `height:auto`), `.fleet-filter input`
  (32→44), `.search-box` (38→44), `.login-form input` (42→44),
  `.wb-field`/`.wb-action-card` text inputs (34→44), `#ws-rows` text
  inputs (30→44), `.wb-maintain-decide` (28→44). Checkboxes keep their
  visual (`.wb-confirm` labels carry the 44px hit area; `#ws-rows` boxes
  20px in ≥44px rows).
- `touch-action: manipulation` on `a,button,input,select,textarea,label`
  removes the tap delay without disabling pan/zoom.
- Press feedback: `transition: opacity/background-color 120ms ease`
  (inside 80–150ms) + opacity-only `:active` (no layout shift);
  `cursor:pointer` added to `.button-quiet`, `.nav-link`,
  `.wb-maintain-decide`, checkboxes.
- Safe areas: `.topbar` sticks at `env(safe-area-inset-top)` with
  matching padding (60px total unchanged); `.sidebar` top padding gains
  the inset at base/850px/560px widths; fixed `.skip-link` offsets gain
  insets; `body` carries left/right/bottom insets; all `env()` calls have
  `0px` fallbacks; both viewport metas gain `viewport-fit=cover`.
- `100vh` → `100dvh` twins (vh kept first as fallback) at all four
  sites: `body`, `.auth-layout`, `.app-shell`, 560px `.auth-layout`.
- `scrollIntoViewRespectingMotion` helper honors
  `prefers-reduced-motion` (`auto` vs `smooth`, null-guarded, degrades
  without `matchMedia`); both call sites (`openMaintainDecision`
  center, `loadWorkbenchDetail` start) route through it; no
  unconditional `{ behavior: "smooth" }` remains (CSS guard alone cannot
  override the explicit JS option).

Evidence at archive (change active for verification, archived after):

| Check | Result |
|---|---|
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; 2 pre-existing warnings only |
| `forge_web_navigation_contract` | **6 passed / 0 failed** |
| `forge_web_manage_deep_link_browser` (real Chromium) | **1 passed / 0 failed** (no regression) |
| `forge_web_workbench_deep_link_browser` (real Chromium) | **1 passed / 0 failed** (no regression) |
| static token audit over shipped files | **19 / 19**: 11 target heights ≥44px + label area, `touch-action`, 120ms transition, `:active` opacity, cursor set, topbar/sidebar/skip-link/body insets with fallbacks, `viewport-fit=cover` ×2, 3 dvh twins, helper + 2 call sites + media query |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **88 passed / 0 failed** active; **87 passed / 0 failed** after archive |
| `git diff --check` | clean |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change adds no `src/` file); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec archive portal-touch-responsive-targets --yes` | `portal-web-ui: update` (+4); **no `--skip-specs`**; archived as `2026-10-08-portal-touch-responsive-targets`; `openspec list` reports **no active changes** |

Pre-existing failures not attributable to this change (recorded, not
fixed — separate harnesses, separate changes): `source-file-size`
(`src/`-only gate; 48/119 both before and after) and the two
`project-runtime` adapter-timeout unresolved items; remediation paths
unchanged from prior slices (src-file splitting program; harness
timeout investigation).

Files changed: `frontend/app.js`, `frontend/index.html`,
`frontend/login.html`, `frontend/styles.css`,
`openspec/specs/portal-web-ui/spec.md` (+4), new
`openspec/changes/archive/2026-10-08-portal-touch-responsive-targets/`
(proposal, design, tasks, delta spec), `HANDOFF.md` (this entry).

Commits: implementation+archive+spec-promotion `d05ed4d`, HANDOFF
evidence (this commit). `openspec list` reports no active changes; no
`current_spec` pointer remains. Nothing pushed.

### portal-focus-route-contrast delivered and archived (2026-10-08)

Slice 1 of the frontend UI/UX audit (P1 accessibility focus+contrast;
slices 2–6 stay QUEUED, not authored). `frontend/` only, vanilla
HTML/CSS/JS; no API/registry/journal/CLI/catalog change.

Fix (`frontend/app.js`, `frontend/index.html`, `frontend/login.html`,
`frontend/styles.css`):

- `renderRoute` moves focus to `#main-content` (`tabindex="-1"`,
  `preventScroll`) on view switches only (`lastRouteView` guard);
  same-view `?project=` reconciliations never steal focus, so
  fleet-manage-deep-link reload/back-forward/login behavior is intact.
- `html { scroll-padding-top: 76px }`, sticky 60px `.topbar`,
  `scroll-margin-top` on the region + `section[id]` (focus-not-obscured).
- `login.html` `color-scheme` `light` → `dark`, paired with the shared
  dark token set.
- `--faint` `#7d8698` → `#8b93a6`: worst pair (badge-chip blend `#202329`)
  4.30:1 → 5.11:1; every `--faint`/`--muted` normal-text pair now ≥5.1:1.
- `--focus: #9aa5ff` defined and wired to all `:focus-visible` rules (two
  sites referenced an undefined `var(--focus)`); login inputs keep
  `border-color` and gain a 2px outline at 8.4:1 on the input background.

Evidence at archive (change active for verification, archived after):

| Check | Result |
|---|---|
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; 2 pre-existing warnings only |
| `forge_web_navigation_contract` | **6 passed / 0 failed** |
| `forge_web_manage_deep_link_browser` (real Chromium) | **1 passed / 0 failed** (no regression) |
| `forge_web_workbench_deep_link_browser` (real Chromium) | **1 passed / 0 failed** (focus move causes no regression) |
| post-change contrast audit (relative-luminance ratios) | all PASS: badge-blend faint 5.11:1, placeholder/action-cli faint 5.60:1, th/updated/security-note faint 6.09:1, sidebar/topbar pairs 6.46–7.94:1, focus indicator 8.21–8.72:1 (needs: 4.5:1 text, 3:1 focus) |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **88 passed / 0 failed** active; **87 passed / 0 failed** after archive |
| `git diff --check` | clean |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change adds no `src/` file), governance-quality **1 ERROR** (pre-existing from the prior change — see below); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec archive portal-focus-route-contrast --yes` | `portal-web-ui: update` (+4); **no `--skip-specs`**; archived as `2026-10-08-portal-focus-route-contrast`; `openspec list` reports **no active changes** |

Pre-existing failure not attributable to this change (recorded, not
fixed — separate change):

- `governance-quality`: `check-spec-governance` flags
  `openspec/specs/fleet-manage-deep-link/spec.md:4` TBD Purpose
  placeholder, stamped by the prior change's archiver after its Gate run
  (its HANDOFF entry records governance-quality pass while active). This
  change touches no canonical spec except `portal-web-ui` (+4 real
  requirements, Purpose intact). Remediation: a follow-up change authoring
  a source-backed Purpose line for the `fleet-manage-deep-link` spec.

Files changed: `frontend/app.js`, `frontend/index.html`,
`frontend/login.html`, `frontend/styles.css`,
`openspec/specs/portal-web-ui/spec.md` (+4), new
`openspec/changes/archive/2026-10-08-portal-focus-route-contrast/`
(proposal, design, tasks, delta spec), `HANDOFF.md` (this entry).

Commits: implementation `d14143c`, HANDOFF evidence (this commit).
`openspec list` reports no active changes; no `current_spec` pointer
remains. Nothing pushed.

### fleet-manage-deep-link delivered and archived (2026-10-08)

Bug: opening `http://127.0.0.1:4173/management?project=alethefy` does not
focus/manage alethefy. Live state (read-only; nothing on the live pair
was changed):

| Fact | Evidence |
|---|---|
| alethefy is NOT registered | live registry holds 7 records (`argoscope`, `demoapp`, `localonly`, `mnemora`, `native-react-preview`, `net-app`, `portdemo42170`); none is alethefy |
| alethefy is observed/unmanaged | `GET /v1/admin/projects` (throwaway mirror of live: registry copy + `FORGE_ADMIN_PROJECTS_ROOT=/home/paul/code`): `management: "observed"`, `source: "published"` (forge-publish-history, 25 such rows), `capabilities: []`, no conflict |
| alethefy is onboardable | sibling exists, no `forge.yaml`, import-decidable (`python-service`, high confidence), workspace candidate `unregistered` + selectable, derived id `alethefy` |
| Fleet Manage action for it | `<a href="/management?project=alethefy">Manage</a>` — correct destination for an unmanaged project |

Browser repro (throwaway mirror on ports 56195/50627, real Chromium;
live pair only read, never written):

- Authenticated `/management?project=alethefy` ticks alethefy with a
  naming notice (parameter handling itself works).
- **Signed-out exact URL reproduces the report**: bounce to `login.html`,
  after sign-in land on `index.html` — `?project=alethefy` lost, alethefy
  never focused. Root cause: the login handoff drops the deep link.
- `/workbench?project=alethefy` boots an empty workbench: no
  `?project=` support there. Managed rows use an "Open" button to bare
  `/workbench` (not shareable, lost on reload).

Fix (`frontend/app.js` only; no API/registry/catalog/CLI change):

- Workbench reads `?project=` on every route render once the fleet has
  populated its selector (`workbenchProjectParam` /
  `applyWorkbenchProjectParam`, `wbAutoParam`/`wbFleetReady` guards):
  managed id selects + loads; unknown id loads nothing + honest notice;
  absent id changes nothing (plain `/workbench` byte-identical behavior).
- Fleet "Open" navigates to `/workbench?project=<id>`; hand selection via
  dropdown/Load routes through the same URL so it stays deep-linkable.
- `/management?project=<registered-id>` `replaceState`-redirects to
  `/workbench?project=<id>`; all other non-selectable/unknown states keep
  the select-nothing notice.
- Login handoff carries `login.html?next=<path+query>` and returns to it
  only when it is a validated same-origin route path (`loginNextTarget`
  + `isRoutePath`); anything else falls back to `index.html`, never
  off-origin.

Evidence at archive (change active for verification, archived after):

| Check | Result |
|---|---|
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; 2 pre-existing warnings only |
| new `forge_web_workbench_deep_link_browser` | **1 passed / 0 failed** (Open-URL + boot, direct boot, reload, unknown id, back/forward across two managed ids, registered redirect, signed-out management + workbench round-trips, hostile `next`) |
| `forge_web_navigation_contract` (extended token test) | **6 passed / 0 failed** |
| `forge_web_manage_deep_link_browser` | **1 passed / 0 failed** (no regression) |
| `forge_web_command_catalog_contract` / `forge_web_maintainer_surface_contract` | **9 / 5 passed, 0 failed** |
| `cargo test --lib command_catalog` / `cargo test --bin forge` | **7 / 7 passed, 0 failed** |
| wider web set: fleet 11, execution 5, delivery-controls 10, management 9, onboarding 10, workbench 11, admin-api 4, portfolio 11, status 10, actions 6 | all passed, 0 failed |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **87 passed / 0 failed** |
| `git diff --check` | clean |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change adds no `src/` file); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| mirror retest of the exact reported URL (signed-out + signed-in, both views, hostile `next`) | all behaviors verified in Chromium |

Pre-existing failures unrelated to this change (recorded, not fixed —
separate harnesses, separate changes):

- `forge_web_workspace_onboarding_browser`: asserts dashboard order
  `workbench < management < commands`, but `#commands-title` no longer
  exists since archived `lifecycle-tracked-project-view` removed the
  catalog page. Remediation: update the harness order assertion to the
  lifecycle-tracked layout.
- `forge_web_project_delivery_browser`: calls `selectOption` on
  `#workbench-project` while the projects view is visible; fails
  identically with pristine `HEAD` `frontend/app.js` (verified via
  stash). Remediation: navigate to `/workbench` (now
  `/workbench?project=<id>`) before selecting.

Manual retest (throwaway mirror; live data untouched):

```sh
# registry copy + fresh admin on the copy only:
cp ~/.local/share/forge/registry.db /tmp/mirror.db
sqlite3 /tmp/mirror.db "DELETE FROM forge_admin; DELETE FROM forge_admin_sessions;"
printf '<pw>' | ./target/debug/forge --registry /tmp/mirror.db identity setup --email o@e.t --password-stdin
FORGE_FRONTEND_ORIGIN=http://127.0.0.1:<WEB> FORGE_ADMIN_PROJECTS_ROOT=/home/paul/code \
  ./target/debug/forge --registry /tmp/mirror.db api serve --bind 127.0.0.1 --port <API> &
./target/debug/forge web serve --bind 127.0.0.1 --port <WEB> --root <frontend-copy-with-fixed-config> &
```

| Step | Expect |
|---|---|
| Signed out → `http://<WEB>/management?project=alethefy` → sign in | returns to that URL, alethefy row ticked + named notice |
| Signed in → `/workbench?project=<managed>` | selector set + detail loaded; reload keeps; unknown id → empty state + `nothing was loaded` notice |
| Signed in → `/management?project=<registered>` | URL becomes `/workbench?project=<id>`, detail loads |
| `login.html?next=https://example.invalid/` → sign in | lands on `index.html`, stays on-origin |

Files changed: `frontend/app.js`,
`tests/forge_web_navigation_contract.rs`, `HANDOFF.md` (this entry),
new `openspec/changes/archive/2026-10-08-fleet-manage-deep-link/`
(proposal, design, tasks, delta spec), new
`tests/browser/workbench-deep-link-check.mjs`,
`tests/forge_web_workbench_deep_link_browser.rs`, promoted canonical
spec `openspec/specs/fleet-manage-deep-link/spec.md` (+4, no
`--skip-specs`). Pre-existing companions kept as found:
`tests/browser/manage-deep-link-check.mjs`,
`tests/forge_web_manage_deep_link_browser.rs`.

Commits: implementation `2fdf0f1`, archive+spec promotion `71e53d8`,
HANDOFF evidence (this commit). `openspec list` reports no active
changes; no `current_spec` pointer remains. Nothing pushed.

### forge-web-navigation-routing delivered and archived (2026-10-08)

`forge-web-navigation-routing` is implemented, verified and archived as
`openspec/changes/archive/2026-10-08-forge-web-navigation-routing`,
creating the `forge-web-navigation-routing` spec (3 requirements),
promoted without `--skip-specs`. Routing choice: **(a) SPA path routing**.
The dashboard is a single application shell; five separate HTML pages would
duplicate the shared sidebar/topbar/scripts five times for zero behavioural
gain. One shell + one view-selection function + additive allowlist entries
is the smaller, robust change for this exact-path server.

- `src/web.rs`: `asset_name` maps `/projects`, `/workbench`, `/management`,
  `/portfolio`, `/delivery` to `index.html`; `/` stays `login.html`;
  exact-match allowlist, no traversal; unit test extended (new arms, unknown
  and traversal `None`).
- `frontend/index.html`: `<base href="/">`; real nav `href`s with
  `data-route`; `#view-projects` wrapper; `#topbar-crumb`; `hidden` on
  non-default views; Data-sources section balanced. Only remaining `href="#"`
  is the a11y skip link.
- `frontend/app.js`: `VIEW_BY_PATH` / `viewForPath` / `renderRoute` /
  `navigateTo`, click interceptor, `popstate` listener, boot-time
  `renderRoute`; fleet "Manage" and workspace hint → `/management`;
  `openInWorkbench` → `/workbench` via `navigateTo`.
- New `tests/forge_web_navigation_contract.rs` (3 tests): each route 200 +
  shell bytes; `/` login bytes; unknown/traversal 404; nav has no `href="#"`
  anchors; router tokens present.
- No CLI path added, no catalog row/count change. `scripts/web.sh`
  unchanged (symlink staging picks up frontend files). Auth/session and
  CSP/security headers unchanged. Login stays at `/`.

Evidence at archive:

| Check | Result |
|---|---|
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; pre-existing warnings only |
| new `forge_web_navigation_contract` | **3 passed / 0 failed** |
| `cargo test --lib command_catalog` / `cargo test --bin forge` | **7 / 7 passed, 0 failed** |
| `plugins_contract` / `catalog_contract` | **9 / 18 passed, 0 failed** |
| `classify_derive_contract` / `classify_apply_contract` | **6 / 5 passed, 0 failed** |
| `web_login_credentials_contract` | **6 passed / 0 failed** |
| `forge_web_command_catalog_contract` / `forge_web_maintainer_surface_contract` | **9 / 5 passed, 0 failed** |
| `forge_web_command_execution_contract` / `forge_web_delivery_controls_contract` / `forge_web_fleet_contract` / `forge_admin_api_contract` | **5 / 10 / 11 / 4 passed, 0 failed** |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **86 passed / 0 failed** |
| `git diff --check` | clean |
| `openspec archive forge-web-navigation-routing --yes` | `forge-web-navigation-routing: create` (+3); **no `--skip-specs`**; archived as `2026-10-08-forge-web-navigation-routing`; `openspec list` reports **no active changes** |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change adds no oversized `src/` file); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec list` | no active changes |

Operator restart commands and exact real URLs (`forge web serve` default
bind/port per `scripts/web.sh`; replace host/port with the banner values):

```sh
scripts/web.sh start
```

| Destination | URL |
|---|---|
| All projects | `http://127.0.0.1:4173/projects` |
| Workbench | `http://127.0.0.1:4173/workbench` |
| Manage projects | `http://127.0.0.1:4173/management` |
| Portfolio | `http://127.0.0.1:4173/portfolio` |
| Delivery | `http://127.0.0.1:4173/delivery` |
| Login (unchanged) | `http://127.0.0.1:4173/` |

### web-login-credential-bootstrap delivered and archived (2026-10-08)

`web-login-credential-bootstrap` is implemented, verified and archived as
`openspec/changes/archive/2026-10-08-web-login-credential-bootstrap`,
creating the `web-operator-credential-bootstrap` spec (3 requirements) and
extending `forge-admin-login` (+2 requirements), promoted without
`--skip-specs`. An operator can now run `scripts/web.sh start` and get a
working login printed on the console; `scripts/web.sh reset-password`
rotates it and prints the new one once.

The implementation adds no new storage and reuses the existing Argon2id
admin store unchanged:

- `src/identity/global.rs`: `pub fn email(db_path)` reads only the `email`
  column (never the hash).
- `src/main.rs`: `identity status` (human `configured:`/`email:` lines or
  `forge-admin-login/1.0.0` JSON with `configured` + `email`);
  `--password-stdin` on `identity setup` and `identity change-password`
  (one stdin line, no terminal, no confirmation; the interactive
  double-entry path is byte-identical); no `--password` value flag exists.
- `scripts/web.sh`: `--admin-email` (default `operator@example.com`),
  `ensure_admin` called at the top of `start`, a new `reset-password`
  subcommand, and a login banner printing URL + email + (only when it just
  created/reset) the password to stdout. The password is never an argv
  element, never written to `.forge/run/state`, never logged. `start` never
  resets an existing account; it points at `reset-password`.
  `FORGE_BIN`/`FORGE_WEB_RUN_DIR`/`FORGE_WEB_LOG_DIR`/`FORGE_WEB_ROOT_DIR`
  let the contract test isolate a throwaway run directory.
- `src/api/command_catalog.rs`: one `identity.status` CLI-only read row;
  pinned row count 233 → 234 with the delta comment.
- `ROADMAP.md`: the operating rule no longer asserts an empty queue while a
  change is active (governance-quality finding introduced by this change).
- `openspec/specs/maintainer-plugin-platform/spec.md`: its archiver TBD
  Purpose is replaced with a source-backed one (the prior archive's
  governance debt, cleared here).

New `tests/web_login_credentials_contract.rs` (6 tests) drives the real
binary and the real script:

| Scenario | Asserts |
|---|---|
| `status_reports_fresh_then_configured_with_email` | fresh JSON `configured:false`/`email:null`; human `configured: false`; after `setup --password-stdin` `configured:true` + email; status never prints the password |
| `setup_password_stdin_works_without_the_secret_in_argv` | piped password becomes the credential (authenticates); `--password <value>` is not a flag; `--help` advertises only `--password-stdin` |
| `change_password_stdin_rotates_and_invalidates_a_prior_session` | old password stops working, new works, a pre-rotation session is revoked |
| `a_short_password_on_stdin_is_refused` | <12 chars refused for setup (no row written) and for rotation (old credential survives) |
| `reset_password_prints_once_and_never_writes_the_password_to_state` | banner email + password; printed password authenticates; `.forge/run/state` never holds it |
| `start_creates_then_recognizes_the_account_without_inventing_a_password` | first `start` prints URL/email/password; second prints URL/email and no password, naming `reset-password` |

Evidence at archive:

| Check | Result |
|---|---|
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; pre-existing warnings only |
| new `web_login_credentials_contract` | **6 passed / 0 failed** |
| `plugins_contract` / `catalog_contract` | **9 / 18 passed, 0 failed** |
| `classify_derive_contract` / `classify_apply_contract` | **6 / 5 passed, 0 failed** |
| `forge_web_command_catalog_contract` / `forge_web_maintainer_surface_contract` | **9 / 5 passed, 0 failed** |
| `cargo test --lib command_catalog` / `cargo test --bin forge` | **7 / 7 passed, 0 failed** |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `node scripts/check-spec-governance.mjs` | **PASS** (after this change) |
| `openspec validate --all --strict --no-interactive` | **85 passed / 0 failed** (before archive) |
| `git diff --check` | clean |
| `openspec archive web-login-credential-bootstrap --yes` | `forge-admin-login: update` (+2), `web-operator-credential-bootstrap: create` (+3); **no `--skip-specs`**; archived as `2026-10-08-web-login-credential-bootstrap`; `openspec list` subsequently reports **no active changes** |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change adds no oversized `src/` file); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout); governance-quality was 1 attributable before the ROADMAP fix and is **pass** after |
| `openspec list` | no active changes |

Manual smoke: `forge identity status` prints `configured: false` on a fresh
registry and `configured: true` + email after setup; `scripts/web.sh
reset-password` prints the banner and the printed password authenticates.

Implementation commit: `362b057`. Nothing pushed. No `current_spec`
pointer remains because no OpenSpec change is active.

### maintainer-plugin-platform delivered and archived (2026-10-08)

`maintainer-plugin-platform` is implemented, verified and archived as
`openspec/changes/archive/2026-10-08-maintainer-plugin-platform`,
creating the `maintainer-plugin-platform` spec (5 requirements) and
promoting it without `--skip-specs`. Forge now observes each registered
project's own compose/CI facts, derives a reviewable classification from
local evidence, publishes the approved set outward through a named
plugin registry, and exposes both the fleet filter row and the
per-project Maintain card in the browser.

The implementation reuses the existing boundaries and changes no
transport:

- `src/catalog/source.rs`: `local_record` populates `compose`/`ci` from
  the project directory via the already-written
  `import::detect_docker` / `import::detect_ci` (now `pub(crate)`),
  replacing the hard-coded `compose: None` and the quality-verdict `ci`.
  A project with neither reports `none`; `available` drives the
  `unavailable` evidence state.
- `src/semantic/derive.rs`: `forge classify derive [TARGET]` reads the
  manifest's declared profile/maturity/target_maturity, the manifest
  runtime language and the README's first heading, and records proposals
  through the unchanged `forge-semantic-proposal/0.1.0` contract with
  evidence and bounded confidence. No model, no network, no project-field
  write. With no persisted GitHub observation in this slice, portfolio
  tags fall back to the local runtime language and the evidence says so
  — the design's declared adapter-absent fallback.
- `src/semantic/apply.rs`: `forge classify apply [TARGET] --confirm` is
  the only new write path. It refuses any non-`Approved` proposal by
  name, refuses any approved value outside
  `ALLOWED_PROPOSED_FIELDS`, and sends one
  `forge-metadata-propose/0.1.0` request in `mode: "pr"` through
  `invoke_metadata_provider`, which shares `run_provider_process` with
  publish (bounded argv, per-run timeout, secret-leak rejection).
- `src/plugins/mod.rs`: the registry over `providers.yaml` (plus an
  optional sibling `plugins:` descriptor block). A descriptor-less plugin
  is a `delivery` plugin, so no existing config changes; an unknown
  capability or kind is `invalid`; a missing command is `unavailable`
  without hiding the others. `forge plugins list` renders it.
- `src/api/maintain.rs` + `src/api/mod.rs` + `src/main.rs`: the
  read-only `GET /v1/admin/projects/{id}/maintain` projection and
  confirm/digest-bound `classify/approve`, `classify/reject` and
  `classify/apply` routes; the catalog admits the admin session cookie
  (no bearer, origin-checked) on `GET /v1/projects/catalog` so the
  browser filter row can call the same Core query.
- `frontend/`: the fleet filter row (language, lifecycle, profile,
  compose, CI, tag) composing with the free-text/source filters, and the
  Maintain card (GitHub observation + freshness or honest
  unavailable-with-reason, derived proposals with per-field
  approve/reject, plugins, one apply) reusing `buildActionControl`.

New contract suites drive the real binary: `catalog_contract` (+1:
`compose_and_ci_are_observed_from_each_registered_project_directory`),
`plugins_contract` (9), `classify_derive_contract` (6),
`classify_apply_contract` (5),
`forge_web_maintainer_surface_contract` (5) and the extended
`forge_web_command_catalog_contract` (9).

Evidence at archive:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | 0 errors; pre-existing warnings only |
| `catalog_contract` / `plugins_contract` | **18 / 9 passed, 0 failed** |
| `classify_derive_contract` / `classify_apply_contract` | **6 / 5 passed, 0 failed** |
| `forge_web_command_catalog_contract` / `forge_web_maintainer_surface_contract` | **9 / 5 passed, 0 failed** |
| `cargo test --lib command_catalog` / `cargo test --bin forge` | **7 / 7 passed, 0 failed** |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **84 passed / 0 failed** |
| `git diff --check` | clean |
| `openspec archive maintainer-plugin-platform --yes` | `maintainer-plugin-platform: create` (+5); **no `--skip-specs`**; archived as `2026-10-08-maintainer-plugin-platform`; `openspec list` subsequently reports **no active changes** |
| `forge gate` (mandatory local run) | **BLOCKED baseline-identical, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security; fail: source-file-size **48 of 119** (pre-existing; this change adds no oversized `src/` file); unresolved: declared-verification, tests (environmental `project-runtime` 60s adapter timeout) |
| `openspec list` | no active changes |

Manual smoke: `forge plugins list` lists the configured jenkins
(delivery, ready) and openpanel (delivery, disabled); `forge classify
derive` on a fixture records `profile`/`lifecycle`/`domain` proposals
with evidence; `forge project list --compose docker+compose` returns the
fixture carrying `compose.yaml`.

Implementation commit: `47b86b1`. Nothing pushed. No `current_spec`
pointer remains because no OpenSpec change is active.

### Two open defects found by browser-driving the dashboard (2026-10-08, NOT fixed)

Driving the running dashboard with Playwright (rather than
reading the source) surfaced two defects that are recorded
here and deliberately **not fixed**. Both are outside the
browser surface, so neither belonged to
`lifecycle-tracked-project-view` or to the follow-up
`2486f9e` UI fix. Each needs its own change.

#### Defect 1 — `GET /v1/admin/projects/{id}` takes ~27 seconds

`build_health` in `src/api/workbench.rs` (around line 261)
runs `run_driftwatch` **and** `run_doctor` inline, in the
request path, every time the workbench opens a project:

```rust
let policy_outcome = run_driftwatch(dir, &DriftWatchConfig::from_env());
match run_doctor(dir, None, observation.as_ref(), Some(&policy_outcome)) { ... }
```

Measured in the browser on `argoscope`:
`performance.getEntriesByType('resource')` reported
**27,242 ms** for that single request. A direct `curl` of
the same authenticated route reported ~6.5 s, so the cost
is real on the server side and varies with what the
project's own toolchain does.

Why this matters to the operator: the frontend's `request()`
helper has no timeout, so opening a project leaves the
workbench looking hung for the better part of half a
minute. This is very likely the dominant cause of the
workbench reading as "broken" rather than as a design
problem.

The honest read is that an external-checker run and a full
doctor pass are **not** read operations and should not be
inline in a GET. Candidate remediations, none chosen yet:
bound the workbench's health projection to the cached
`doctor` result already in the registry; move the live pass
behind an explicit operator action with its own progress
state; or run it once and cache per `(project, commit)`.
Whichever is picked must keep the existing contract
honest — a health row must never claim a pass it did not
compute.

#### Defect 2 — there is no way to take a project to its death

The operator asked to track a project "from new to publish
or to death". The "death" half does not exist:

```
$ forge --help | grep -iE "retire|archive|deprecate|remove|delete|unregister|tombstone"
(no output — exit 1)
```

There is no `retire` / `archive` / `deprecate` /
`unregister` CLI command, and no matching admin route. The
lifecycle-tracked workbench defines a **Retire** action
group, and that group is therefore **structurally empty**:
`lifecycleStageFor()` maps any future `retire.*` /
`deprecate.*` command into it, and no such command exists
yet. The group is hidden rather than faked, because
rendering a button that cannot do anything is worse than
its absence.

What "death" has to mean is an open product decision, not
an implementation detail: a project reaches its end
retrospectively (archived upstream, superseded, abandoned,
deleted) or when its operator decides to stop maintaining
it. That decision needs the owner's ruling before a change
package can be authored, because the answer determines the
schema (a tombstone row? a `lifecycle` column on
`projects`? a journal-only record?), whether the project
disappears from the fleet or stays visible as retired, and
what happens to its share/publication records.

Neither defect is attributable to any delivered change, and
neither blocks the operator's other work. Both are
recorded here so the next change can pick one up.

### lifecycle-tracked-project-view delivered and archived (2026-10-08)

`lifecycle-tracked-project-view` is implemented, verified and
archived as
`openspec/changes/archive/2026-10-08-lifecycle-tracked-project-view`,
creating the `lifecycle-tracked-project-view` spec (three
new requirements, one removed). The standalone "Command
catalog (reference)" browser page is gone; the per-project
workbench is now lifecycle-tracked.

The workbench's first card is **Lifecycle**: maturity
(`L0`…`L4`), target maturity, profile, path, last update,
plus a one-line operator-gloss of the maturity level. The
second card is **Manage this project**, which renders one
button per available action, grouped by lifecycle stage:
**Adopt** (e.g. `forge new`, `forge import`, `forge register`,
`forge workspace sync`, `forge graduation`), **Day-to-day**
(`forge feature`, `forge spec`, `forge upgrade`, `forge doctor`,
`forge kit`, `forge test`, `forge commit`), **Release**
(`forge release`, `forge deploy`, `forge publish`,
`forge delivery`), and **Retire** (future `retire.*` /
`deprecate.*` commands; the default stage for an unmatched
catalog id is Day-to-day). Each button runs the existing
preview → confirm flow; no new confirmation machinery.

The implementation is a strict refactor of the browser
surface; no CLI, no JSON API, no registry/journal schema,
no Rust module changes. The catalog JSON endpoint
`/v1/admin/commands` stays; the workbench is the only
consumer. The `forge-web-command-catalog` /
`forge-web-command-execution` /
`forge-web-command-workflows` spec files are unchanged.

- `frontend/index.html`:
  - "Commands" nav link removed.
  - `<section id="commands">` (the catalog page) removed.
  - Workbench reorder: Lifecycle → Manage this project →
    Upgrade workflow → read-only cards (manifest, health,
    status, delivery, journal evidence).
  - Lifecycle card has a `<dl id="wb-lifecycle">` with
    `wb-lifecycle-maturity` / `-target` / `-profile` /
    `-path` / `-updated` cells.
- `frontend/app.js`:
  - `renderCommands` / `showCommandsError` / `renderWorkflows`
    removed.
  - `renderLifecycle(manifest)` and `describeMaturity(level)`
    added.
  - `LIFECYCLE_STAGES` and `lifecycleStageFor(command)` added;
    `renderProjectActions` groups executable rows by stage.
  - `loadCommands` keeps the JSON fetch but no longer
    populates the catalog-page filter widgets.
- `frontend/styles.css`:
  - `.command-table` / `.command-id` / `.command-path` /
    `.command-guidance` / `.depth-1` / `.depth-2` removed.
  - `.wb-lifecycle` / `.lifecycle-list` /
    `.wb-actions-grouped` / `.wb-actions-group` /
    `.wb-actions-head` / `.wb-actions-help` added.
- `tests/forge_web_command_catalog_contract.rs`:
  - `frontend_ships_the_catalog_as_labels_never_execution`
    replaced by
    `frontend_uses_the_catalog_for_per_project_buttons_not_a_reference_page`,
    which asserts the catalog page is gone and the workbench
    groups the catalog rows as buttons (8 tests stay
    green).

Evidence at archive:

| Check | Result |
|---|---|
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors (pre-existing warnings only) |
| `cargo test --bin forge` (catalog parity) | **7 passed / 0 failed** |
| `forge_web_command_catalog_contract` (8, including the renamed test) / `forge_web_command_workflows_contract` (5) / `forge_web_command_execution_contract` (5) / `forge_admin_api_contract` (4) | all green; the renamed test asserts the new shape |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **83 passed / 0 failed** (before and after archive) |
| `git diff --check` | clean |
| `openspec archive lifecycle-tracked-project-view --yes` | `lifecycle-tracked-project-view: create` (+3, -1); **no `--skip-specs`**; archived as `2026-10-08-lifecycle-tracked-project-view`; `openspec list` subsequently reports **no active changes** |
| `forge gate` (mandatory local run) | unchanged from prior changes: `governance-quality` pass; `source-file-size` fail (50/119 — pre-existing); `tests` / `declared-verification` unresolved (environmental adapter timeouts); **0 attributable** to this change |
| `openspec list` | no active changes |

Implementation commit: `2d5fae1`. Nothing pushed. No
`current_spec` pointer remains because no OpenSpec change
is active.

### remove-api-ui delivered and archived (2026-10-08)

`remove-api-ui` is implemented, verified and archived as
`openspec/changes/archive/2026-10-08-remove-api-ui`,
creating the `remove-api-ui` spec (one new requirement).
The in-process HTML portal at `/ui/*` on the API listener
(port 8765) is gone. The standalone web UI on `frontend/`,
served by `forge web serve` on port 4173, is now the
operator's only browser-facing control plane.

The implementation is a pure deletion: the JSON API the web
UI calls (`/v1/...`) is unchanged, every CLI subcommand is
unchanged, and the registry/journal schema is unchanged.

- `src/api/ui/` (5 files, 3,335 lines plus `static/`) is
  deleted: `mod.rs`, `auth.rs`, `data.rs`, `render.rs`,
  `routes.rs`. The `FleetRow`, `FleetListView` and
  `load_fleet_list` types and function the JSON API fleet
  projection depended on move to a new
  `src/api/fleet_data.rs` module (227 lines, slimmer than
  the prior `data.rs` — only the parts the JSON API uses).
- `src/api/mod.rs` drops the `pub mod ui;`, the
  `Route::UiFleet` / `Route::UiSignIn` /
  `Route::UiAuthCallback` / `Route::UiSignOut` /
  `Route::UiProjectDetail` / `Route::UiProjectPublish` /
  `Route::UiProjectPortfolio` / `Route::UiStudioProject`
  enum variants, the 6 `("GET|POST", ["ui", ...])`
  pattern arms, the auth-bypass `if !matches!(route,
  Route::Ui*)` block, the `ui::routes::handle_*` dispatch
  arms, and the matching `Route::Ui*` arms in the
  `required_permission` and `authorize` match expressions.
- `src/api/fleet.rs` is updated to import `load_fleet_list`
  from the new `super::fleet_data` module instead of the
  deleted `super::ui::data`.
- `tests/portal_ui_contract.rs` (1,431 lines),
  `tests/portal_browser_a11y.rs` and
  `tests/forge_portal_frontend_contract.rs` are deleted.
- `openspec/specs/forge-web-human-dashboard` is deleted.
  The other `forge-web-*` specs stay — they describe the
  JSON API routes the web UI calls (`/v1/admin/...`),
  not the deleted `/ui/*` HTML portal.
- `forge identity init` (global admin password), the
  `forge identity validate` manifest schema check, the
  `forge identity challenge / callback / list / inspect /
  terminate` OIDC CLI subcommands, the `forge portal
  dashboard / view` CLI command, and the `forge` CLI
  surface at large are all unchanged.

Evidence at archive:

| Check | Result |
|---|---|
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build --workspace` | 0 errors (pre-existing warnings only) |
| `cargo test --bin forge` (catalog parity at 229) | **7 passed / 0 failed** |
| `forge_admin_api_contract` (4) / `forge_web_command_catalog_contract` (8) / `forge_web_command_execution_contract` (5) / `forge_web_workspace_onboarding_contract` (10) / `portal_contract` (10) / `portal_cross_surface` (6) / `identity_contract` (19) / `studio_api_contract` (2) / `api_contract` (11) | all green; no regression in the surviving suites |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **82 passed / 0 failed** (before and after archive) |
| `git diff --check` | clean |
| `openspec archive remove-api-ui --yes` | `remove-api-ui: create` (+1); **no `--skip-specs`**; archived as `2026-10-08-remove-api-ui`; `openspec list` subsequently reports **no active changes** |
| `forge api serve` (PID 689308, port 8765) live | `GET /ui` → `route-not-found`; `GET /healthz` → `{"status":"ok",...}`; `GET /v1/admin/session` → admin-origin-rejected (expected) |
| `forge web serve` (PID 700173, port 4173) live | `GET /` → `200 text/html`; web UI loads and exercises the JSON API |
| `openspec list` | no active changes |

Implementation commit: `8b5d99d` (and the prior
`f5c9c5e`). Nothing pushed. No `current_spec` pointer
remains because no OpenSpec change is active.

### source-file-size-remediation delivered and archived (2026-10-08)

`source-file-size-remediation` is implemented, verified and
archived as `openspec/changes/archive/2026-10-08-source-file-size-remediation`,
creating the `source-file-size-remediation` spec (two
requirements). The change is a **bounded first slice**: it
converts `tests/kit_contract.rs` (the only `tests/` file over
the 1000-line cap) to `tests/kit_contract/`, a directory of
focused submodules, with the 60 `#[test]` functions (1
`#[ignore]`) preserved verbatim and `cargo test --test
kit_contract` reporting the same 59 passed / 1 ignored count
as before the move.

Honest scope of the gate impact: the `forge gate`
`source-file-size` check evaluates `src/**/*.rs` only, not
`tests/**`. Before this change, **50 of 119** `src/` files
were over 1000 lines. After this change, **50 of 119** — the
count is unchanged, because this slice targets the test
target only. The change paves the way: it ships a working
submodule pattern (Cargo's directory-based test targets with
`main.rs` as the entry, `pub(crate)` visibility since the
integration crate is its own root) and authors a follow-on
roadmap in `design.md` §9 that lists the 51 oversized `src/`
files (and 4 oversized `tests/` files) with the natural split
axis for each. The roadmap is one-file-per-future-change
work, not a single-package bolt-on.

The implementation reuses Cargo's directory-based test
target mechanism and changes no other boundary:

- `tests/kit_contract.rs` (2376 lines) → `tests/kit_contract/`
  with `main.rs` (file doc, shared helpers, `FeedRestore` RAII
  struct, `mod` declarations) plus 8 submodules by concern:
  `registration` (14 tests), `feed` (13), `prewiring` (4),
  `tokens_and_assets` (8), `tampering` (4, one of which is
  `#[ignore]`), `classification` (4), `upgrade` (8),
  `manifest` (5). Total: 60 `#[test]` functions, identical
  bodies to the prior file. Every function body is copied
  verbatim; no "while I'm here" cleanup.
- Visibility changes from `pub(super)` to `pub(crate)`
  because a Rust integration test crate is its own root: the
  previous `super::` would refer above the crate root and
  would fail to compile.
- No `mod ...;` declaration in `src/lib.rs` or `src/main.rs`
  changes. No public symbol, journal column, route, CLI
  argument, catalog row, env var, `--version` output, or
  test name changes.

New `openspec/changes/source-file-size-remediation/`
(proposal / design / tasks / delta spec) and the follow-on
roadmap in `design.md` §9 cover all 55 remaining oversized
files. Each is a single, well-bounded, future OpenSpec
change.

Evidence at archive:

| Check | Result |
|---|---|
| `cargo fmt` then `cargo fmt --check` | clean |
| `cargo build` | 0 errors; pre-existing warnings only |
| new `cargo test --test kit_contract` | **59 passed / 0 failed / 1 ignored** (identical to before the move) |
| `cargo test --bin forge` (catalog parity at 229) | **7 passed / 0 failed** |
| `forge_web_command_catalog_contract` (8) / `forge_web_project_management_contract` (9) / `forge_admin_api_contract` (4) / `forge_web_fleet_contract` (11) / `portal_ui_contract` (39) | all green; no regression |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **82 passed / 0 failed** (before and after archive) |
| `git diff --check` | clean |
| `openspec archive source-file-size-remediation --yes` | `source-file-size-remediation: create` (+2); **no `--skip-specs`**; archived as `2026-10-08-source-file-size-remediation`; `openspec list` subsequently reports **no active changes** |
| `forge gate` (mandatory local run) | `governance-quality` **pass**; `source-file-size` **fail (50 of 119) — unchanged from baseline** because the gate evaluates `src/` only and this change targets `tests/`; the 2 environmental adapter timeouts (`declared-verification`, `tests`) remain pre-existing; **0 attributable** to this change (the 9 new test files are all under 1000 lines) |
| `openspec list` | no active changes |

Implementation commit: `eb934e8`. Nothing pushed. No `current_spec`
pointer remains because no OpenSpec change is active.

### Governance placeholder debt cleared (2026-10-08)

The openspec archiver stamps a TBD Purpose line into every new canonical
spec. The gate's `governance-quality` check pins every one of those
markers as an error, so 14 web admin/portal specs delivered between
2026-10-06 and 2026-10-07 carried stale `## Purpose: TBD - created by
archiving change X. Update Purpose after archive.` placeholders. This
commit authors a one-sentence Purpose line for each, summarizing what
its Requirements actually deliver, drawn from the spec text itself:

| Spec | Authored Purpose |
|---|---|
| `forge-admin-login` | Authenticate the single Forge-wide administrator with an Argon2id-hashed credential, bounded session cookie, and CLI-managed password lifecycle. |
| `forge-web-command-catalog` | Expose an authenticated, versioned web catalog of every CLI command with truthful availability, structured execution blocks, and no generic shell execution. |
| `forge-web-command-execution` | Execute handler-backed CLI commands from the browser under the same confirm-and-digest discipline the CLI uses, with no shell or argv interpolation. |
| `forge-web-delivery-controls` | Plan, confirm, and track browser-triggered repository or external delivery mutations with honest provider outcomes and server-side credential isolation. |
| `forge-web-portfolio-controls` | Manage Forge-owned portfolio metadata and surface cross-project evidence with honest source states, freshness, and privacy thresholds. |
| `forge-web-project-actions` | Run `feature remove`, `feature upgrade`, and `spec apply` from the browser as catalog-rendered, confirm-and-digest-bound Core delegations with no subprocess or provider execution. |
| `forge-web-project-delivery` | Surface read-only project delivery status and execute health-gated preflight, stage, promote, and Hermora retry under confirm-and-digest discipline. |
| `forge-web-project-deployment` | Plan and apply project deploys from the browser under confirm-and-digest discipline, delegating to the in-process deploy engine with server-side credentials. |
| `forge-web-project-fleet` | Aggregate the Forge-self record, every local project, configured external sources, and recent publish history into a single authenticated fleet view. |
| `forge-web-project-management` | Create, import, and register projects from the browser via confirm-and-digest-bound Core delegations that resolve paths only from server-side state. |
| `forge-web-project-publish` | Plan and apply provider publishes from the browser under confirm-and-digest discipline with server-side provider resolution and honest outcome reporting. |
| `forge-web-project-release` | Plan and apply project releases from the browser under confirm-and-digest discipline, delegating to the in-process release engine with server-side credentials. |
| `forge-web-project-status` | Surface read-only project status and a fleet readiness summary from the in-process doctor, checker, and readiness projection with honest sub-check states. |
| `forge-web-project-workbench` | Provide project-scoped browser workflows that plan before every write, execute only typed Core contracts, and show partial failures honestly. |

Evidence:

| Check | Result |
|---|---|
| `node scripts/check-spec-governance.mjs` | **0 findings** (was 14 ERROR) |
| `openspec validate --all --strict --no-interactive` | **81 passed / 0 failed** |
| `git diff --check` | clean |
| `forge gate` (mandatory local run) | `governance-quality` now **pass** (0 of 5 in scope); other 2 fails (`source-file-size` 50/119, `tests`/`declared-verification` environmental adapter timeouts) remain pre-existing — see the `forge-workspace-sync` entry above for the follow-up boundary |

Commit: `70bb702`. Nothing pushed. No `current_spec` pointer remains
because no OpenSpec change is active.

### forge-workspace-sync delivered and archived (2026-10-08)

`forge-workspace-sync` is implemented, verified and archived as
`openspec/changes/archive/2026-10-08-forge-workspace-sync`, creating the
`forge-workspace-sync` spec (two requirements). The operator can now bring
a whole workspace root into the registry in one terminal invocation:
`forge workspace sync [ROOT]` (default `.`) walks the immediate children,
registers manifest directories, adopts decidable directories through the
unchanged `inspect_import` / `adopt_import` / `derive_project_id` /
`Registry::register` Core path, reports already-registered directories
`already` without rewriting, and names every skipped/failed entry with a
typed reason. Reruns over an unchanged root report everything `already`;
reruns after adding siblings onboard only the new ones.

The implementation reuses the existing Core adoption/registration
machinery and changes no other boundary:

- `src/import/mod.rs`: new `WorkspaceSyncEntry` / `WorkspaceSyncSummary` /
  `WorkspaceSyncReport` plus `sync_workspace(&mut Registry, &Path)` and
  the versioned `WORKSPACE_SYNC_CONTRACT = "forge-workspace-sync/0.1.0"`
  constant. Sorted immediate-child walk; `hidden` / `not-a-directory` /
  `symlink` / `unreadable` / `undecidable` / `ambiguous` skip with a
  reason; `id-collision` / `path-collision` / `manifest-invalid` /
  Core-error fail typed; one counts-only `workspace.sync` journal row.
- `src/main.rs`: new top-level `Workspace` command with `WorkspaceCommands::Sync`
  (`root: PathBuf`, default `.`), dispatched before the generic
  `match &cli.command` so the per-directory report still prints when
  some directories failed; renders the human table and the
  `forge-workspace-sync/0.1.0` JSON envelope (`--format json` /
  `--format ndjson`); exit 0 iff no directory failed (skips never fail
  the run).
- `src/api/command_catalog.rs`: one new `workspace.sync` row added as
  `cli_only` with `REASON_LOCAL_FS`; pinned count moves 227 → 229
  (`workspace` + `workspace.sync`).
- `frontend/` and the browser onboarding flow are unchanged: the
  per-directory previews/confirms/digests the workspace panel ships stay
  authoritative for browser-driven work; this change gives the operator
  a one-shot, scriptable terminal counterpart.

New `tests/workspace_sync_contract.rs` (7 tests) drives the real
`forge` binary against a throwaway fixture root and registry:

| Scenario | Asserts |
|---|---|
| `sync_onboards_a_mixed_workspace` | manifest / decidable / pre-registered register or report `already`; hidden / ambiguous / undecidable / not-a-directory / symlink / invalid-manifest skip or fail with their named reason; summary `synced 2, already 0, skipped 5, failed 1`; registry holds the onboarded + adopted + pre-registered set; an adopted directory's `forge.yaml` is written |
| `rerun_is_idempotent_and_converges_new_siblings` | first run `synced 2, already 0, skipped 1, failed 0`; second run `synced 0, already 2, skipped 1, failed 0`, no new `register` journal row, adopted manifest bytes byte-unchanged; adding a third sibling reports `ok new-app new-app` and `synced 1, already 2, skipped 1, failed 0` |
| `failures_do_not_stop_siblings` | an identity collision (`taken-id` pre-registered elsewhere) reports `failed taken-id id-collision` with no manifest written, an invalid manifest reports `failed bad-app manifest-invalid`, a healthy sibling still reports `ok good-app good-app`; the pre-registered `taken-id` row's path is unchanged; exit non-zero |
| `json_shape_is_stable` | `--format json` envelope parses with `contract: forge-workspace-sync/0.1.0`, canonical `root`, exactly 8 entries (one per child), and `summary: {ok: 2, already: 0, skipped: 5, failed: 1}` |
| `skips_only_root_exits_zero` | root with only ambiguous / undecidable / hidden / non-directory entries exits 0 and reports `synced 0, already 0, skipped 4, failed 0` |
| `empty_root_reports_zero_counts` | empty root exits 0 with `synced 0, already 0, skipped 0, failed 0` |
| `missing_root_is_a_typed_error_and_touches_nothing` | missing ROOT returns non-zero and the typed `path-unavailable` error on stderr without creating the registry |

Evidence at archive:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | clean (0 errors; pre-existing warnings only) |
| new workspace sync contract | **7 passed / 0 failed** |
| `forge_web_command_catalog_contract` (8) / `forge_web_project_management_contract` (9) / `forge_web_fleet_contract` (11) / `forge_admin_api_contract` (4) / `portal_ui_contract` (39) | all green; no catalog or web regression |
| `--bin forge` (catalog parity at 229, integrity) | **7 passed / 0 failed** |
| `--lib import::` (16) / `--lib api::` (49) | green |
| `catalog_contract` (17) | green |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **81 passed / 0 failed** (before and after archive) |
| `git diff --check` | clean |
| `openspec archive forge-workspace-sync --yes` | **20/20 tasks**; `forge-workspace-sync: create` (+2); **no `--skip-specs`**; archived as `2026-10-08-forge-workspace-sync`; `openspec list` subsequently reports **no active changes** |
| `forge gate` (mandatory local run) | **BLOCKED baseline-identical** — 5 pass (build, placeholder-threshold, product-code-boundary, repository, security); 2 fail (governance-quality 14 errors, source-file-size 50/119), 2 unresolved (declared-verification, tests — environmental adapter timeout); **0 attributable** to this change (the 2 new modules are well under the 1000-line cap and this change's promoted spec carries a real Purpose line, not the archiver's TBD marker) |
| `openspec list` | no active changes |

Implementation commit: `622a0a3`. Nothing pushed. No `current_spec`
pointer remains because no OpenSpec change is active.

### forge-web-human-dashboard delivered and archived (2026-10-07)

`forge-web-human-dashboard` is implemented, verified and archived as
`openspec/changes/archive/2026-10-07-forge-web-human-dashboard`,
creating the `forge-web-human-dashboard` spec (three requirements). The
dashboard now speaks human: fleet rows lead with display names and one
plain status line (no source/lifecycle/access/evidence code columns);
previews, results and confirmations read as sentences with no rendered
hashes, digests, op-ids or full revisions (values stay in JS state and
wire bodies; staged delivery confirmations arrive pre-filled); failures
are plain instructions with a next step; onboarding success re-fetches
the fleet in place with results still visible. The visual system is one
dark command-center theme meeting WCAG 2.2 AA (browser contrast checks
green). Routes, codes, statuses, digests, journals and catalog rows are
unchanged.

Evidence at archive:

| Check | Result |
|---|---|
| `cargo fmt --check` / `cargo build` / `git diff --check` | clean / 0 errors / clean |
| portal frontend (new readability/token/no-hash/in-place tests) | **12 passed / 0 failed** |
| delivery browser (prefill rewrite, real Chromium) | **1 passed / 0 failed** |
| management / onboarding contract / onboarding browser | **9 / 10 / 1 passed, 0 failed** |
| portal UI / fleet / admin API | **39 / 11 / 4 passed, 0 failed** |
| `lib api::` | **49 passed / 0 failed** |
| `check-openspec-change-names` | PASS |
| `openspec validate --all --strict` | **81 passed / 0 failed** |
| `openspec archive` | archived as `2026-10-07-forge-web-human-dashboard`, canonical create (+3), no `--skip-specs` |
| `forge gate` (mandatory local run) | **BLOCKED baseline-identical** — 14 governance errors with 0 attributable (evidence-store run 10), size 50/119 unchanged, 2 environmental unresolved |
| `openspec list` | only parked `forge-workspace-sync` remains (proposal/design/tasks/spec authored, unimplemented) |

Implementation commit: `17e923f`. Nothing pushed. No `current_spec`
pointer remains because no change is actively being worked.

### forge-web-command-workflows delivered and archived (2026-10-07)

`forge-web-command-workflows` is implemented, verified and archived as
`openspec/changes/archive/2026-10-07-forge-web-command-workflows`,
creating the `forge-web-command-workflows` spec (three requirements). The
dashboard is now ordered around work instead of reference: fleet →
workbench → management (creation + workspace onboarding) → portfolio →
delivery → command catalog (reference, last), with the sidebar nav in the
same order. Workspace discovery auto-runs on dashboard load with an “N of M
workspace directories are not yet onboarded” line under the fleet linking
to the onboarding panel. Bulk onboarding Shortcut: selections of any size
are previewed in 25-item chunks (one combined plan, one digest per chunk)
and applied sequentially under a single confirmation tick; a refused chunk
stops the run with completed results kept and a re-preview prompt. No API,
catalog, Core or journal change — `frontend/` only.

Evidence at archive:

| Check | Result |
|---|---|
| `cargo fmt --check` / `cargo build` | clean / 0 errors |
| extended onboarding browser drive (27-dir, two-chunk) | **1 passed / 0 failed** ×3 runs (order, auto-discovery, chunked confirm, fleet-appears, keyboard, contrast, no-path) |
| portal frontend / portal UI / catalog / management / workbench / execution | **8 / 39 / 8 / 9 / 11 / 5 passed, 0 failed** |
| `cargo test --bin forge` / `--lib api::` / delivery browser | **7 / 49 / 1 passed, 0 failed** |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **79 passed / 0 failed** |
| `git diff --check` | clean |
| `openspec archive forge-web-command-workflows --yes` | archived as `2026-10-07-forge-web-command-workflows`; canonical `forge-web-command-workflows: create` (+3); no `--skip-specs` |
| `forge gate` (mandatory local run, twice) | first run: governance 15 errors incl. 1 attributable (this change's promoted spec carried the archiver's TBD Purpose marker) → fixed with a real Purpose line → second run: **14 errors, 0 attributable**; size/unresolved identical to baseline (see below) |
| `openspec list` | no active changes |

Gate attribution (both runs recorded in `.driftwatch` runs 8–9):

- `governance-quality`: 14 errors before and after; all are the stale TBD
  Purpose markers in older specs. The single attributable finding (this
  change's own spec marker) was fixed before completion.
- `source-file-size`: 50 failing files before and after (50/117 → 50/119
  evaluated; the +2 are this change's new test/script files, all far under
  the limit). Remediation path: follow-up splitting `admin.rs`/`mod.rs`
  remainder + the other oversized files.
- `declared-verification` / `tests` UNRESOLVED: environmental
  `project-runtime` 60s adapter timeouts, identical across all runs
  including pre-session baselines.

Implementation commit: `256b986` (includes the one-line Purpose fix for
the workspace-onboarding spec that the gate flagged). Nothing pushed. No
`current_spec` pointer remains because no OpenSpec change is active.

### forge-web-workspace-onboarding delivered and archived (2026-10-07)

`forge-web-workspace-onboarding` is implemented, verified and archived as
`openspec/changes/archive/2026-10-07-forge-web-workspace-onboarding`,
creating the `forge-web-workspace-onboarding` spec (three requirements). The
dashboard can now discover every sibling directory under the
operator-configured project root and bulk import/register a selection
through preview → confirm → per-item results. Discovery re-reads the root
live on every request, so an expanding workspace needs no code, list or
configuration change; the fleet itself stays database-driven (onboarding
writes registry records, the fleet reads those records, never the
filesystem). No concrete host folder appears in code, specs, tests or
responses — the root is runtime-only operator configuration
(`FORGE_ADMIN_PROJECTS_ROOT`, documented in the README portal section).

Delivered routes (session-gated, exact-origin `/v1/admin`):

- `GET /v1/admin/workspace/candidates?limit=&cursor=` — bounded, sorted,
  paginated candidate view per immediate child directory: leaf name,
  derived id, manifest presence, suggested profile/confidence, live
  registration state (`unregistered`/`registered`/`id-collision`/
  `ambiguous`/`undecidable`/`manifest-invalid`/`unreadable`) and next
  action. Hidden entries, files and symlinks are skipped; symlink escapes
  are refused.
- `POST /v1/admin/workspace/onboard` — bulk preview (per-item plans +
  64-hex digest, no write) → `409` on mismatched digest or blocked items →
  confirmed apply through the unchanged `adopt_import` /
  `Registry::register` Core functions with honest per-item results (`202`
  all ok, `207` partial) plus a counts-only `admin.workspace.onboard`
  parent row. The digest binds the reviewed root plus the exact per-item
  plans, so a directory set that changes between preview and confirm is
  refused safely. Non-kebab leaves onboard via explicit `id` override;
  at most 25 items per batch.

The dashboard gains a task-oriented “Workspace onboarding” panel
(discover/refresh, candidate table with selection and per-item profile/id
overrides, preview, confirm, per-item results, fleet reload) rendered
text-only with labelled controls and a live region; single-item
new/import/register controls are unchanged. All five delivery verbs from
the prior change keep working through the same panel patterns.

Required remediation inside this change: `src/api/admin.rs` (3,822 lines)
is split — project management moves to `src/api/project_management.rs`
(563 lines), workspace onboarding to `src/api/workspace.rs` (728 lines),
`admin.rs` shrinks to 2,589. Pure move, proven by unchanged green suites.
Remaining `admin.rs`/`mod.rs` size debt stays tracked under the gate
remediation path below.

Evidence at archive:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | 0 errors; one pre-existing warning (`ShareSurface` unused import) |
| new onboarding contract | **10 passed / 0 failed** |
| new Chromium browser flow | **1 passed / 0 failed** (discover → select → preview → confirm → fleet) |
| catalog / execution / fleet / management / workbench | **8 / 5 / 11 / 9 / 11 passed, 0 failed** |
| admin API / portal frontend / portal UI | **4 / 7 / 39 passed, 0 failed** |
| `cargo test --bin forge` | **7 passed / 0 failed** |
| `cargo test --lib api::` | **49 passed / 0 failed** |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **78 passed / 0 failed** |
| `git diff --check` | clean |
| `openspec archive forge-web-workspace-onboarding --yes` | archived as `2026-10-07-forge-web-workspace-onboarding`; canonical `forge-web-workspace-onboarding: create` (+3); no `--skip-specs` |
| `forge gate` (mandatory local run) | **BLOCKED** — 5 pass (build, placeholder-threshold, product-code-boundary, repository, security); 2 fail, 2 unresolved, all pre-existing/environmental, zero new failures attributable to this change (see below) |
| `openspec list` | no active changes |

Gate attribution (new rule: record verdict, block only on failures
attributable to the change):

- `source-file-size` FAIL is pre-existing in cause: 50 failing files at both
  baseline and final runs (50/117 → 50/119 evaluated; the +2 are this
  change's two new modules, both under 1,000 lines). `admin.rs` shrank
  3,822 → 2,589 here. Remediation path: follow-up package to continue
  splitting `admin.rs`/`mod.rs` plus the other 48 oversized files.
- `governance-quality` FAIL shows the identical 14 errors at baseline and
  final runs with no other repo activity between them. Remediation path:
  triage the 14 findings in a follow-up governance package.
- `declared-verification` / `tests` UNRESOLVED are environmental: the
  `project-runtime` adapter exceeded its 60s timeout in both runs.
  Remediation path: re-run in CI / investigate the adapter environment.

Implementation commit: `65b679b`. Nothing pushed. No `current_spec` pointer
remains because no OpenSpec change is active.

### forge-web-project-delivery delivered and archived (2026-10-07)

`forge-web-project-delivery` is implemented, verified and archived as
`openspec/changes/archive/2026-10-07-forge-web-project-delivery`, creating the
`forge-web-project-delivery` spec (four requirements) and updating the
`forge-web-command-catalog` and `forge-web-command-execution` specs. The
workbench can now drive the complete evidence-gated delivery sequence through
the session-gated, exact-origin `/v1/admin` boundary: read-only
`GET /v1/admin/projects/{id}/delivery/status`, plus confirm-then-digest
`POST` routes for `delivery/preflight`, `delivery/stage`,
`delivery/promote` and `delivery/hermora-retry`. Every mutation delegates to
the unchanged `delivery::handlers` verb used by the CLI and preserves staged
health gating, revision binding, journaling and idempotency.

The implementation preserves the established security and honesty boundaries:

- `src/api/admin.rs`: five route consts, staged descriptors, registered-revision
  resolution from a read-only registry, typed previews, confirm/digest gates,
  Core delegation and scrubbed typed errors. The browser supplies only the
  documented staged confirmation fields; provider, path, binary, argv, host,
  revision source and secret values are never accepted.
- Stage accepts a strictly parsed operation id; promotion requires an exact
  40-hex revision; Hermora retry requires an HTTP(S) URL without embedded
  credentials and an `env:`-prefixed environment-variable reference. Hermora
  URL/reference values participate in the digest but are not echoed in
  previews.
- Missing sessions, non-JSON mutations, hostile/unmanaged ids, missing
  providers, failed providers, unhealthy stages and missing healthy
  deployments are typed before or through Core, never reported as success.
- Absolute project paths, provider paths, URL credentials and
  credential-shaped values are scrubbed or refused without echo.
- `src/api/mod.rs`: five route variants, router arms and all
  session/permission/dispatch/authorize lists.
- `src/api/command_catalog.rs`: all five delivery rows are now `web`; the four
  mutations carry executable blocks, and all five routes join the implemented
  route list.
- `src/delivery/projection.rs`: narrow connected-Hermora decoding correction.
  Connected detail contains `"reason": null`; the old `BTreeMap<String,
  String>` decoder discarded the whole object and lost `site_id`, so the phase
  could not become `hermora-connected`. Optional detail fields are now decoded
  as JSON values. No orchestration, journal or adapter behavior changed.
- `frontend/app.js` + `frontend/index.html`: a workbench delivery card shows
  phase, revision, latest staged evidence and the next required confirmation,
  including the operation id or full revision needed by the following control.
  Mutations reuse generic catalog-driven controls; all rendering is text-only
  with loading, empty, unavailable, permission, error, confirmation, success
  and blocked states.

New `tests/forge_web_project_delivery_contract.rs` (12 tests) uses a
throwaway registry/project and hermetic OpenPanel/Hermora stubs. It covers
anonymous/non-JSON refusal, side-effect-free status, hostile/unmanaged ids,
missing-provider honesty, missing/stale prerequisite conflicts, malformed
confirmation refusal, preview/mismatch behavior, failed-provider reporting,
unhealthy-stage promotion refusal, missing-adapter behavior, the full
draft-to-connected run with idempotent stage replay, and catalog agreement.

New `tests/forge_web_project_delivery_browser.rs` plus
`tests/browser/delivery-workbench-check.mjs` drive the shipped login and
workbench in real Chromium: keyboard entry, project selection, draft delivery
status, preview/confirm execution of all four verbs, final
`hermora-connected` phase, focus behavior, delivery-card contrast and proof
that neither the absolute fixture root nor Hermora secret reference is
rendered.

Evidence at archive:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | 0 errors; existing warnings only |
| new delivery contract | **12 passed / 0 failed** |
| new Chromium browser flow | **1 passed / 0 failed** |
| catalog / execution / deployment / release / publish / actions / workbench | **8 / 5 / 8 / 8 / 9 / 6 / 11 passed, 0 failed** |
| admin API / publish fleet / delivery controls / frontend / portal UI | **4 / 7 / 10 / 6 / 39 passed, 0 failed** |
| catalog / delivery / cross-surface / publish contracts | **17 / 17 / 9 / 11 passed, 0 failed** |
| `cargo test --lib delivery::` | **21 passed / 0 failed** |
| `cargo test --bin forge` | **7 passed / 0 failed** |
| `cargo test --lib api::` | **49 passed / 0 failed** |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **77 passed / 0 failed** |
| `git diff --check` | clean |
| `openspec archive forge-web-project-delivery --yes` | archived as `2026-10-07-forge-web-project-delivery`; canonical `forge-web-project-delivery: create` (+4), catalog/execution updates; no `--skip-specs` |
| `openspec list` | no active changes |

Implementation commit: `c376744`. Nothing pushed. No `current_spec` pointer
remains because no OpenSpec change is active.

### forge-web-project-publish delivered and archived (2026-10-07)

`forge-web-project-publish` is implemented, verified and archived as
`openspec/changes/archive/2026-10-07-forge-web-project-publish`, creating the
`forge-web-project-publish` spec (three requirements) and modifying one
`forge-web-command-catalog` requirement and one `forge-web-command-execution`
requirement. The browser can now plan and run a provider publish for one
managed project through the established session-gated, exact-origin
`/v1/admin` boundary: read-only
`GET /v1/admin/projects/{id}/publish/plan` and confirm-then-digest
`POST /v1/admin/projects/{id}/publish`. Both resolve the project directory,
provider id, provider configuration and committed revision only from
server-side state; both delegate to the unchanged in-process
`publish::providers::{load_config, select_provider, invoke_provider}` path
used by the CLI. This closes the provider-publish follow-on the deployment
package deferred. OpenPanel delivery with health-gated promotion remains the
explicitly named `forge-web-project-delivery` follow-on.

The implementation preserves the existing security and honesty boundaries:

- `src/api/admin.rs`: route consts plus `publish_target`, `publish_revision`,
  `publish_descriptor`, `publish_plan_view`, `journal_publish_phase`,
  `publish_plan` and `publish_write`. No provider, revision, path, binary,
  argv, host, SSH target, credential, stage list or shell text is accepted from
  the browser; the apply body reads only `confirm` and `plan_digest`.
- Missing sessions, non-JSON mutation bodies, hostile/unmanaged ids, missing
  provider configuration and digest mismatches are refused before any provider
  call or journal write. Provider spawn/timeout/non-zero/invalid-response
  failures are journaled as `failed` and returned as typed
  `503 publish-provider-unavailable`; unhealthy provider output is returned
  with its real status/health/evidence and `healthy: false`.
- Absolute project/provider/config paths, SSH targets, credentials and adapter
  binaries are scrubbed with project-dir/provider-binary secrets plus
  `redact_local_paths`.
- `src/api/mod.rs`: `AdminProjectPublishPlan` / `AdminProjectPublish`
  variants, router arms and all session/permission/dispatch/authorize lists.
- `src/api/command_catalog.rs`: bare `publish` moves from
  `provider_required` to a zero-parameter `web_exec` row pointing at the apply
  route; both new routes join `IMPLEMENTED_WEB_ROUTES`, with pinned in-source
  coverage.
- `frontend/`: unchanged. The shipped generic `buildActionControl` already
  renders the zero-parameter `execution` row as a preview → confirm → run
  control with no inputs and no bespoke command/path field.

New `tests/forge_web_project_publish_contract.rs` (9 tests) uses a throwaway
registry/project and hermetic stub provider executable:

| Scenario | Asserts |
|---|---|
| anonymous/non-JSON refused | 401/415 before any provider call or journal row |
| plan preview | 200 path-free plan, 64-hex digest, no invocation or journal |
| missing prerequisite | typed 409 names only the variable/configuration, no value/id/path |
| hostile/unmanaged id | typed 400/404 with no echo, no invocation or journal |
| apply preview/wrong digest | 200 preview then 409 mismatch, both with no invocation or journal |
| confirmed matching digest | one provider invocation, one `publish` row with provider-reported state |
| provider failure | typed 503, journaled `failed`, never success |
| unhealthy provider response | 202 with real status/health/evidence and `healthy: false` |
| catalog agreement | `publish` is `web` with an empty-parameter `execution` block |

Evidence at archive:

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | 0 errors; existing warnings only |
| new publish contract | **9 passed / 0 failed** |
| catalog / execution / deployment / release / actions / workbench | **8 / 5 / 8 / 8 / 6 / 11 passed, 0 failed** |
| admin API / publish fleet / portal UI / publish contract | **4 / 7 / 39 / 11 passed, 0 failed** |
| `cargo test --bin forge` | **7 passed / 0 failed** |
| `cargo test --lib api::` | **48 passed / 0 failed** |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **76 passed / 0 failed** |
| `git diff --check` | clean |
| `openspec archive forge-web-project-publish --yes` | archived as `2026-10-07-forge-web-project-publish`; canonical `forge-web-project-publish: create` (+3), catalog/execution updates; no `--skip-specs`; one non-blocking proposal-length warning |
| `openspec list` | no active changes |

Implementation commit: `d4876de`. Nothing pushed. No `current_spec` pointer
remains because no OpenSpec change is active.

### forge-web-project-release delivered and archived (2026-10-07)

`forge-web-project-release` is implemented, verified and archived as
`openspec/changes/archive/2026-10-07-forge-web-project-release`, creating the
`forge-web-project-release` spec (three requirements) and modifying one
`forge-web-command-catalog` requirement and one `forge-web-command-execution`
requirement. The project's release workflow now runs end to end through the same
session-gated, exact-origin, JSON-only `/v1/admin` boundary the deploy and
lifecycle actions use: a read-only
`GET /v1/admin/projects/{id}/release/plan?version=` and the confirm-then-digest
`POST /v1/admin/projects/{id}/release`. Both delegate to the in-process
`release::engine::{prepare_release, apply_release}` the CLI runs — no subprocess,
provider or PTY surface crosses into the browser. This closes the release
follow-on the deployment package deferred.

The implementation reuses the engine unchanged and stays within the established
patterns:

- `src/api/admin.rs`: new route consts `ROUTE_ADMIN_RELEASE_PLAN` /
  `ROUTE_ADMIN_RELEASE`; helpers `parse_release_version` /
  `release_version_from_query` / `release_gate` / `release_request` /
  `release_plan` / `release_write` / `run_release_plan_view` plus
  `release_plan_view` / `release_report_view` projections and a
  `typed_release_error` mapper. The `version` parameter is a typed semver parsed
  with `Semver::parse` and normalized to `.label()` (a missing or non-semver
  value is a typed `400`); the path-free descriptor/digest is
  `authoring_digest(json!({"project_id": id, "version": label}))`, and the
  browser never supplies release stages — `config.stages` decides. No absolute
  project path, adapter binary, git credential or secret is serialized
  (`redact_local_paths` + `scrub_json`, and `ReleaseReport.state_path` is
  omitted).
- Boundary shape mirrors `deploy_write`: 415 JSON gate, `guarded` session cookie
  `forge_admin_session`, `validate_project_id` → 400 with no echo,
  `registry.inspect(id)` → 404, version parse → 400 before any digest; no-`confirm`
  returns a 200 preview + fresh digest and writes nothing; a mismatched digest is
  a typed `409 admin-digest-mismatch` with a refreshed digest and still no write;
  only a matching `confirm: true` reaches
  `run_with_operation(db, "release", id, || apply_release(...))`, journals
  `admin.project.release` and returns a 202 path-free typed report. Git/subprocess
  /stage failures stay typed, journaled and honest (real stage state, never a
  fake success).
- `src/api/mod.rs`: `Route::AdminProjectReleasePlan` and
  `Route::AdminProjectRelease` variants wired into the router arms and all four
  exhaustive match lists (short-circuit, `required_permission`, dispatch,
  authorize) so the five- and six-segment admin release paths share the existing
  `/v1/admin/projects/{id}/*` session, permission and CORS group.
- `src/api/command_catalog.rs`: `release.prepare` recatalogued from
  `not_yet_web` to a `web` read row pointing at the plan route;
  `release.apply` recatalogued as a `web_exec` row (POST, `confirm_required:
  true`, `digest_bound: true`, `risk: remote_write`, parameter
  `version: string` required) pointing at the apply route. `release.list` and
  `release.inspect` stay honest `not_yet_web`. The pinned `executable_ids` set
  moves 6 → 7, the catalog row count stays at 227, and the in-source allowlist +
  pins and the external `tests/forge_web_command_catalog_contract.rs` allowlist /
  `web_ids` set are updated to include the two new routes and their web ids.
- `frontend/app.js` + `frontend/index.html`: untouched. The shipped
  `buildActionControl` already renders any `execution` block generically, so the
  release row joins the same preview → confirm → typed result control as the
  lifecycle and deploy actions.

New `tests/forge_web_project_release_contract.rs` (8 tests) drives the real
`handle()` against a throwaway git fixture with a local bare `origin` (no real
network push) and a failing `FORGE_NOTES_BIN` stub for the honest-failure path:

| Scenario | Asserts |
|---|---|
| `catalog_reports_release_plan_and_apply_as_web_with_an_execution_block` | both rows are `web`; plan row has no `execution`; apply row has route/method/parameter/confirm/digest/risk; `executable_ids` set contains `release.apply` |
| `anonymous_and_non_json_are_refused_before_any_core_call` | anonymous plan → 401; non-JSON body → 415; no `.forge` / registry write before any Core call |
| `hostile_id_is_refused_and_unmanaged_id_is_not_found` | traversal id → 400 typed `admin-invalid-project-id` with no echo; unmanaged id → 404; no path/token leakage |
| `missing_or_non_semver_version_is_refused` | absent or non-semver `version` → typed 400, never a digest |
| `plan_route_reads_no_write_and_returns_a_path_free_preview` | GET plan → 200; 64-hex `plan_digest`; `.forge` and registry journal byte-unchanged; no absolute path or adapter binary in the body |
| `apply_preview_writes_nothing_and_wrong_digest_is_refused` | no-`confirm` → 200 with fresh digest and zero writes; mismatched digest → 409 `admin-digest-mismatch` + refreshed digest, still no write |
| `confirmed_apply_with_a_local_origin_reaches_the_core_handler` | confirmed + matching digest → 202 path-free report; `release` operation journaled; project path and adapter binary never serialized; local `origin` receives the tag with no network push |
| `adapter_failure_is_reported_honestly_never_as_success` | a failing `FORGE_NOTES_BIN` stub still returns 202 (HTTP success for a confirmed matching digest) while the report's `healthy: false` + failed stage + journal entry remain observable; the failure is never papered over |

Evidence at archive:

| Check | Result |
|---|---|
| `cargo build` | clean (0 errors; 2 pre-existing warnings, none added) |
| `cargo fmt --check` | clean |
| `cargo test --test forge_web_project_release_contract` | **8 passed / 0 failed** |
| `cargo test --test forge_web_command_catalog_contract` / `forge_web_command_execution_contract` / `forge_web_project_deployment_contract` / `forge_web_project_actions_contract` / `forge_web_project_workbench_contract` | **8 / 5 / 8 / 6 / 11 passed, 0 failed** |
| `cargo test --test forge_admin_api_contract` / `forge_web_fleet_contract` / `forge_web_portfolio_controls_contract` / `forge_web_delivery_controls_contract` / `forge_portal_frontend_contract` | **4 / 11 / 11 / 10 / 5 passed, 0 failed** |
| `cargo test --test portal_ui_contract` / `catalog_contract` / `release_contract` / `release_cross_surface` | **39 / 17 / 10 / 4 passed, 0 failed** |
| `cargo test --bin forge` | **7 passed / 0 failed** (catalog integrity/coverage/`executable_ids`/route-allowlist) |
| `cargo test --lib api::` | **47 passed / 0 failed** |
| `node scripts/check-openspec-change-names.mjs` | PASS (before and after archive) |
| `openspec validate --all --strict --no-interactive` | **75 passed / 0 failed** (before and after archive) |
| `git diff --check` | PASS |
| `openspec archive forge-web-project-release --yes` | **21/21 tasks**; `forge-web-project-release: create` (3 added) and `forge-web-command-catalog: modify` (1) + `forge-web-command-execution: modify` (1); **no `--skip-specs`**; archived as `2026-10-07-forge-web-project-release`; `openspec list` subsequently reports **no active changes** |

Two commits: backend routes + catalog + specs + tests, then this handoff. Nothing
pushed.

### forge-web-project-status delivered and archived (2026-10-07)

`forge-web-project-status` is implemented, verified and archived as
`openspec/changes/archive/2026-10-07-forge-web-project-status`. It adds a new
canonical `forge-web-project-status` spec (4 requirements) and updates the
`forge-web-command-catalog` spec. The session-gated admin surface can now
answer the operator's "find the project status" ask read-only: a per-project
status projection and a fleet-wide readiness summary.

- `src/api/status.rs` (new): `GET /v1/admin/projects/{id}/status` and
  `GET /v1/admin/status` under the existing session cookie + exact-origin CORS
  + JSON-only gate. The registry is opened strictly read-only
  (`Registry::open_read_only`); the project root is resolved server-side by the
  workbench's validated-id `resolve`, never from a browser path. Sub-checks
  reuse the in-process planes only — `run_doctor` (adapter `policy_outcome` is
  `None`; the DriftWatch adapter is never invoked), `governance::inspect` +
  `checker::build_document`, and `Manifest::load_from_dir` + `inspect_profile`
  (the native readiness matrix is never invoked). States are
  `healthy`/`issues`/`stale`/`unavailable` reduced
  `issues > unavailable > stale > healthy`; the fleet path counts every
  registered project and bounds the detail list at 200 with `truncated: true`.
- `src/api/mod.rs` / `src/api/admin.rs`: two `Route` variants, router arms,
  permission/short-circuit/unreachable match lists, and the two `guarded`
  handle arms. `src/api/workbench.rs`: `resolve`/`Resolved`/`redact_local_paths`
  widened to `pub(super)` (visibility only).
- `src/api/command_catalog.rs`: both routes join `IMPLEMENTED_WEB_ROUTES`;
  `check` moves `cli_only` → `web` at the project-status route and
  `fleet.status` repoints to the fleet route (row count stays 227;
  `readiness.*` stay `cli_only`). `tests/forge_web_command_catalog_contract.rs`
  allowlist and `web_ids` extended.
- `frontend/app.js` + `frontend/index.html`: a workbench "Project status" card
  (`#wb-status`) and a dashboard fleet-readiness tile
  (`#fleet-readiness`/`#fleet-*` counts), `textContent` only, no new sink.

New `tests/forge_web_project_status_contract.rs` (10 tests) drives the real
`handle()` against a throwaway registry and covers all ten design §7 scenarios
(anonymous 401 both routes; hostile origin 403; hostile/path-bearing ids
refused with no echo; unmanaged 404; a healthy rust-web project all-`healthy`
with a real git origin; a missing build definition `issues`; a removed root
`unavailable` with path-free reasons; fleet counts summing to total; read-only
proof via unchanged manifest bytes + journal length; catalog + frontend
surface). Pass counts: status 10, command-catalog 8, workbench 17, management
4, admin-api 9, portal-frontend 5, portal-ui 11, catalog 39, `--lib api::` 47,
`--bin forge` 7. `cargo fmt --check`,
`node scripts/check-openspec-change-names.mjs` and
`openspec validate --all --strict --no-interactive` (74 specs) are clean.

### forge-web-project-management delivered and archived (2026-10-07)

`forge-web-project-management` is implemented, verified and archived as
`openspec/changes/archive/2026-10-07-forge-web-project-management`. It adds a
new canonical `forge-web-project-management` spec (3 requirements) and updates
the `forge-web-command-catalog` and `forge-web-command-execution` specs. The
session-gated admin surface can now create, import and register projects from
the browser, closing the operator's "i can still not manage project" tier.
Release and provider publish are named follow-on packages
(`forge-web-project-release`, `forge-web-project-publish`); `upgrade` was
already delivered by the workbench routes and needed no new work. This is the
bounded first slice, stated explicitly in the design decision ledger.

The change lifts the three Core authoring operations the CLI already owns and
does not fork them:

- `src/api/admin.rs`: `POST /v1/admin/projects/{new,import,register}` under the
  existing session cookie + exact-origin CORS + JSON-only 415 gate. Each route
  is a single-request preview/confirm gate: no `confirm` returns a read-only,
  path-free descriptor with a 64-hex `plan_digest`; a mismatched digest is a
  typed `409 admin-digest-mismatch`; only a matching `confirm: true` reaches
  `run_with_operation`, which delegates to `generate` / `adopt_import` /
  `Registry::register` and journals `admin.project.{new,import,register}`.
- The browser names a location only as one validated kebab `project`; the
  server joins it to the unset-by-default `FORGE_ADMIN_PROJECTS_ROOT`. An unset
  root is a typed `409 admin-prerequisite`, never a silent write to
  `FORGE_WORKSPACE_ROOT`. No browser path, binary, argv, host or credential is
  ever accepted, and every response is scrubbed of absolute paths and
  credentials. Failures are typed and journaled, never a fake success.
- `src/api/mod.rs`: the three `Route` variants, router arms and all four match
  lists. `src/api/command_catalog.rs`: `new` / `import` / `register` move from
  `cli_only` to `web_exec` with typed parameters (total row count stays 227).
- `src/api/workbench.rs`: a global, id-less `web` row is honestly reported
  not-actionable inside a single project, carrying a reason that points at the
  dashboard's project-management section.
- `frontend/app.js` + `frontend/index.html`: a dashboard-level "Create or adopt
  a project" section rendered from the catalog's global `execution` rows;
  `textContent` only, no new sink.

New `tests/forge_web_project_management_contract.rs` (9 tests) drives the real
`handle()` against a throwaway registry and covers all ten design §7 scenarios
(anonymous 401 / non-JSON 415; hostile name 400 with no echo; unset root 409;
path-free preview + wrong-digest 409; confirmed register/import/new 202 with
journal rows and no path leak; honest journaled failure; catalog rows). Pass
counts: workbench 11, management 9, command-catalog 8, command-execution 5,
project-actions 6, admin-api 4, portal-frontend 5, portal-ui 39, catalog 17,
`--lib api::` 47, `--bin forge` 7. `cargo fmt --check`,
`node scripts/check-openspec-change-names.mjs` and
`openspec validate --all --strict --no-interactive` (73 specs) are clean.

### forge-web-publish-fleet delivered and archived (2026-10-07)

`forge-web-publish-fleet` is implemented, verified and archived as
`openspec/changes/archive/2026-10-07-forge-web-publish-fleet`, adding one
requirement to the canonical `forge-web-project-fleet` spec. The web fleet
now projects the **local publish journal**, so the projects the operator
ships to the Mac server but never registered locally appear in the portal's
project list instead of being silently absent. This is tier 1 of the
operator's three-part gap report ("the project list is wrong", "i can still
not manage project", "find the project status"); tiers 2 and 3
(`forge-web-project-management`, `forge-web-project-status`) remain
unstarted.

The change is a pure read + aggregation extension, no new mutation,
subprocess, network or credential surface:

- `src/registry/mod.rs`: new `PublishedOperation` + bounded
  `Registry::latest_publishes(limit)` — one `GROUP BY project_id` over
  `kind IN ('publish','publish.github')` selecting each project's newest
  `op_id`, ordered newest-first with a `LIMIT`. Legacy rows with `NULL`
  phase columns project as absent, never guessed.
- `src/api/fleet.rs`: new `RowSource::Published` / `PublishProjection`; a
  `publish: Option<PublishProjection>` field on `CandidateRow`; a
  `redact_local_paths` copy (mirroring the workbench discipline) plus
  `crate::policy::redact_credentials` on every `detail`; `parse_healthy` /
  `parse_stages` for the `healthy=`/`stages=` tokens; `read_published`
  honours `FORGE_PUBLISH_HISTORY` (default **on**; `0`/`false`/`off`
  disables) and `FORGE_PUBLISH_HISTORY_LIMIT` (default 200, clamped
  `1..=1000`). A published id that is already registered (or the self id)
  is **merged** into its managed row, so no false identity conflict is
  introduced and the row keeps `inspect`; only published-only ids become
  new `observed` rows. `WEB_FLEET_CONTRACT_VERSION` bumped
  `forge-web-fleet/0.1.0` → `0.2.0`; `summary.by_source` gains `published`.
- `frontend/app.js` + `frontend/index.html`: a `Published (Mac)` source
  label, badge, and filter option, plus a publish chip on the row
  (state + health; run id / revision / stages / redacted detail in the
  tooltip). No bespoke control; the renderer stays catalog-driven.

New `tests/forge_web_publish_fleet_contract.rs` (7 tests) drives the real
`handle()` against a throwaway registry seeded through the public registry
API (`record_publish_phase`): empty-journal available/0; published-only
observed row with no capability; registered+published stays managed and
merged with `inspect` retained and no conflict; newest publish wins;
`FORGE_PUBLISH_HISTORY=0` reports unconfigured and contributes nothing;
`detail` absolute paths become `[local path]`; anonymous is 401 with no
rows. `forge_web_fleet_contract` (11) stays green because its fixtures
write no publish operations.

Live smoke against a **copy** of the operator's real registry (never the
original): the copy's 3,109 publish rows / 25 distinct projects projected
to 25 observed rows with their latest `done`/`failed` state and
`healthy`/`stages`, the `published` source reported `available, count 25`,
and zero `/home/`, `/Users/`, or temp paths in the response.

### forge-web-project-deployment delivered and archived (2026-10-07)

`forge-web-project-deployment` is implemented, verified and archived as
`openspec/changes/archive/2026-10-07-forge-web-project-deployment`, creating
the `forge-web-project-deployment` spec (three requirements) and modifying
one `forge-web-command-catalog` requirement and one
`forge-web-command-execution` requirement. The project's deploy workflow now
runs end to end through the same session-gated, exact-origin, JSON-only
`/v1/admin` boundary the lifecycle actions use: a read-only
`GET /v1/admin/projects/{id}/deploy/plan` and the confirm-then-digest
`POST /v1/admin/projects/{id}/deploy`. Both routes delegate to the in-process
`deploy::engine::{prepare_deploy, apply_deploy}` the CLI runs — no
subprocess, no provider, no PTY surface crosses into the browser; the
adapter step is read off stdout via `FORGE_DEPLOYER_BIN` exactly as the
existing CLI flow does it. The proposal's two follow-on packages
(`forge-web-project-release`, `forge-web-project-publish`) were explicitly
deferred by the proposal and remain unparked.

The implementation reuses the engine unchanged and stays within the
established patterns:

- `src/api/admin.rs`: three new helpers — `deploy_id_gate`, `deploy_plan`,
  `deploy_write` — share the same `guarded`/`is_json`/digest
  `/run_with_operation` shape as `authoring_write`; new `plan_view` /
  `report_view` projections, a `typed_deploy_error` mapper, and `scrub_json`
  + `redact_local_paths` so an absolute project path or the adapter binary
  can never leave the boundary. `redact_local_paths` exempts self-authored
  `/v1/...` route tokens (no real filesystem path begins with `/v1/`) and
  recurses over every projected string.
- `src/api/mod.rs`: `Route::AdminProjectDeployPlan` and
  `Route::AdminProjectDeploy` arms share the global-session cookie gate,
  exact-origin CORS, the `415` non-JSON gate and the route-permission
  /authorize/handle groups with the existing `/v1/admin/projects/{id}/*`
  family.
- `src/api/command_catalog.rs`: two new consts
  (`WEB_ROUTE_ADMIN_DEPLOY_PLAN`, `WEB_ROUTE_ADMIN_DEPLOY`) plus
  `IMPLEMENTED_WEB_ROUTES` entries; `deploy.plan` recatalogued as a `web`
  read pointing at the plan route; `deploy.apply` recatalogued as a
  `web_exec` row (POST, `confirm_required: true`, `digest_bound: true`,
  `risk: remote_write`, parameter `target: string` optional). The pinned
  `executable_ids` set moves 5 → 6, the catalog count stays at 227, and
  the strict allowlist + `web_ids` set in
  `tests/forge_web_command_catalog_contract.rs` is updated to include the
  two new routes and their web ids.
- `frontend/app.js`: untouched. The shipped `buildActionControl` already
  renders any `execution` block generically; the deploy row joins the same
  preview → confirm → typed result control as the lifecycle actions.

New `tests/forge_web_project_deployment_contract.rs` (8 tests) drives the
real `handle()` over the same in-process registry and admin cookie the
other admin tests use. The two stub deployers are inlined as
`ADAPTER_OK_BODY` / `ADAPTER_FAIL_BODY` shell scripts (no
`tests/fixtures/` file), so the test is hermetic and self-contained:

| Scenario | Asserts |
|---|---|
| `catalog_reports_deploy_plan_and_apply_as_web_with_an_execution_block` | both rows are `web`; plan row has no `execution`; apply row has route/method/parameters/confirm/digest/risk; `executable_ids` set contains `deploy.apply` |
| `anonymous_and_non_json_are_refused_before_any_core_call` | anonymous plan → 401; non-JSON body → 415; no `.forge` / registry write before any Core call |
| `hostile_id_is_refused_and_unmanaged_id_is_not_found` | `..%2F..%2Fetc` id → 400 typed `admin-invalid-project-id` with no echo; unmanaged id → 404; no path/token leakage in body |
| `plan_route_reads_no_write_and_returns_a_path_free_preview` | GET plan → 200; 64-hex `plan_digest`; on-disk `.forge` and registry journal byte-unchanged; response contains no absolute path or adapter binary |
| `apply_preview_writes_nothing_and_wrong_digest_is_refused` | no-`confirm` → 200 with fresh digest and zero registry/journal writes; mismatched digest → 409 `admin-digest-mismatch` + refreshed digest, still no write |
| `confirmed_apply_with_stub_deployer_reaches_the_core_handler` | confirmed + matching digest → 202 path-free report; `apply` stage `delivered`; project path and adapter binary never serialized |
| `empty_target_resolves_to_the_manifest_default` | omitted `target` resolves to the manifest's `default: home` and the plan reflects it |
| `adapter_failure_is_reported_honestly_never_as_success` | a stub returning `apply_status: failed` still returns 202 (the route always succeeds at the HTTP level for a confirmed matching digest) but the report's `healthy: false` + `apply` stage `failed` + journal entry is observable; the failure is never papered over as success |

Honest scope: the headless-Chromium browser drive for the deploy row
specifically is left **UNVERIFIED** — the row joins the same
`buildActionControl` render path exercised by the prior
`forge-web-project-actions` Playwright drive, so a fresh drive would
re-cover the same control. The HTTP/JSON contract the browser would call
is pinned at the contract test layer.

Evidence at archive:

| Check | Result |
|---|---|
| `cargo build` | clean (0 errors; 2 pre-existing warnings, none added) |
| `cargo fmt --check` | clean |
| `cargo test --test forge_web_project_deployment_contract` | **8 passed / 0 failed** |
| `cargo test --test forge_web_command_catalog_contract` / `forge_web_command_execution_contract` / `forge_web_project_actions_contract` / `forge_web_project_workbench_contract` / `forge_admin_api_contract` / `forge_web_fleet_contract` / `forge_web_portfolio_controls_contract` / `forge_web_delivery_controls_contract` / `forge_portal_frontend_contract` | **8 / 5 / 6 / 11 / 4 / 11 / 11 / 10 / 5 passed, 0 failed** (catalog allowlist + `web_ids` set updated) |
| `cargo test --test portal_ui_contract` / `identity_contract` / `deploy_contract` / `deploy_cross_surface` | **39 / 19 / 17 / 5 passed, 0 failed** |
| `cargo test --bin forge` | **7 passed / 0 failed** (catalog integrity/coverage/`executable_ids`/route-allowlist) |
| `node scripts/check-openspec-change-names.mjs` | PASS (before and after archive) |
| `openspec validate --all --strict --no-interactive` | **72 passed / 0 failed** (before and after archive) |
| `git diff --check` | PASS |
| `openspec archive forge-web-project-deployment --yes` | **21/22 tasks**, warning for the 1 UNVERIFIED browser drive honored via `--yes`; `forge-web-project-deployment: create` (3 added) and `forge-web-command-catalog: modify` (1) + `forge-web-command-execution: modify` (1); **no `--skip-specs`**; archived as `2026-10-07-forge-web-project-deployment`; `openspec list` subsequently reports **no active changes** |

Post-archive test-stability fix: re-running the new contract test target
post-archive exposed a 1-in-5 race on `FORGE_DEPLOYER_BIN` between
`confirmed_apply_with_stub_deployer_reaches_the_core_handler` (which sets
the OK adapter) and `adapter_failure_is_reported_honestly_never_as_success`
(which sets the FAIL adapter) when they ran in parallel. The
`handle_with_adapter` helper already cleared the env var on entry and
restored it on exit, but two helpers in two threads can still see each
other's set/restore inside the read. The repo's established
`SERIAL: Mutex<()>` pattern (`forge_web_fleet_contract.rs`,
`studio_preview_contract.rs`) is now applied here — every test in the
binary takes the lock for its full body, so the two `handle_with_adapter`
tests are now serialized and the env-var read is deterministic. **10/10
re-runs clean** after the fix; the contract and the test bodies are
unchanged, only the test-harness synchronization is added.

**Program completion:** with this change archived, `openspec list` reports
no active changes and the web command-center program — global admin
portal, project fleet, command catalog, project workbench, project
actions, portfolio controls, delivery controls and project deployment —
is complete.

### forge-web-project-actions delivered and archived (2026-10-07)

`forge-web-project-actions` is implemented, browser-verified and archived as
`openspec/changes/archive/2026-10-07-forge-web-project-actions`, creating the
`forge-web-project-actions` spec (three requirements) and modifying one
`forge-web-command-catalog` requirement. It closes the gap the user reported
("the command inventory ... i need also it can work, not just a list") at the
Core-only tier they chose: the browser now actually executes the registered-
project lifecycle writes whose operation is already an in-process Core function
keyed by a validated project id — `forge feature remove`, `forge feature
upgrade` and `forge spec apply` — alongside the two authoring commands delivered
by the previous change.

The catalog row, not bespoke frontend wiring, drives each action: a `web_exec`
row serializes a structured `execution` block (route, method, ordered typed
`parameters` with name/kind/required, `confirm_required`, `digest_bound`, risk)
and `frontend/app.js` renders a generic preview → confirm → run control from it,
with no free-text command/path/argv field. `admin.rs` widened the single
`Authoring` enum to `{FeatureAdd, FeatureRemove, FeatureUpgrade, SpecGenerate,
SpecApply}` (no parallel implementation), gates each command's fields into a
path-free canonical descriptor, and on a matching digest delegates to
`remove_feature` / `upgrade_feature` / `apply_routing` — the same Core handlers
the CLI runs. No subprocess, provider or PTY surface crosses into the browser.

While running the full suite, a pre-existing catalog↔Clap drift surfaced: the
`identity change-password` and `identity generate-password` commands added by
`forge-identity-password-management` had no catalog rows, so
`catalog_has_exactly_one_row_per_clap_path` (a `--bin` test) was already red on
the delivered HEAD; both commands' authoring routes were covered but that test
had never been run. This change adds both as truthful `cli_only` rows (a new
`REASON_LOCAL_SECRET` for the generate-password stdout secret) and updates the
`catalog_covers_every_clap_path` count to 227, restoring the catalog↔Clap
parity invariant. A second pre-existing red the fuller run then exposed was
`artifact_baseline_contract::reported_manifest_changelog_versions_agree`: its
`changelog_newest()` helper took the first `## [` heading — the standard
Keep-a-Changelog `## [Unreleased]` bucket — as the versioned entry, so it
compared `0.1.0` against `Unreleased`. The real invariant (Cargo.toml == newest
*released* CHANGELOG version == `forge --version`, all `0.1.0`) holds; the helper
now skips headings that name no version.

Evidence: `cargo build` 0 errors; `cargo fmt --check` clean;
`node scripts/check-openspec-change-names.mjs` PASS;
`openspec validate --all --strict --no-interactive` 71 items pass. New
`tests/forge_web_project_actions_contract.rs` (6) asserts 401/415 before any
Core call, preview-writes-nothing with a path-free 64-hex digest, wrong-digest
409 refusal, confirmed-run reaching the Core handler, typed 400/404 with no echo
for hostile/unmanaged ids, and that the three lifecycle rows are `web` with an
`execution` block; the catalog contract (8), command-execution contract (5) and
`--bin forge` catalog parity/integrity (7) are green. A headless-Chromium run
against a throwaway temp registry (never the user's real registry) signed in,
opened a managed project, rendered the five catalog-driven action cards, then
preview → confirm → run for `feature.remove` (HTTP 202 and the `auth` feature
actually removed from the temp manifest), `feature.upgrade` (reached Core,
returned a typed 400 the page surfaced without crashing) and `spec.apply` (HTTP
202); zero app console errors, zero uncaught page errors, zero network failures,
and no absolute filesystem path in the actions UI. A full `cargo test` run now
reaches every integration target (≈1899 passed) because this change cleared the
catalog↔Clap drift that had fail-stopped the suite earlier; the one failure it
surfaces is unrelated and pre-existing: `portal_browser_a11y` expects the legacy
server-rendered `/ui` to answer 200/202/401 with the shared shell, but an
unauthenticated `GET /ui` now 303-redirects to `/ui/sign-in` (a legacy-portal
change that predates this one). That oracle belongs to the separate
`portal-accessible-responsive-ui` capability and is left for its own
remediation, not rewritten silently here. Two commits: implementation +
catalog/tests, then frontend panel + archive + this handoff. Nothing pushed.

### forge-web-command-execution delivered and archived (2026-10-07)

`forge-web-command-execution` is implemented, browser-verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-command-execution`, promoting one
modified `forge-web-command-catalog` requirement and three new
`forge-web-command-execution` requirements. It closes the requirement.md §36
gap the user reported ("the web UI is not reflect the command, and cannot do the
forge cli command things") for the project authoring commands that already have
typed in-process Core handlers: `POST /v1/admin/projects/{id}/feature`
(`forge feature add` → `handle_add_feature`) and
`/v1/admin/projects/{id}/spec` (`forge spec generate` →
`handle_generate_spec`) are now session-gated, structured-field-only, two-step
preview-then-confirm routes. A no-`confirm` request returns a path-free
descriptor and a SHA-256 `plan_digest` and writes nothing; a confirmed request
runs the Core handler only when the supplied digest still matches the reviewed
fields, else a fresh digest is returned and nothing is written. There is no
generic shell/argv/path endpoint. The `feature.add` and `spec.generate` catalog
rows moved `not_yet_web` → `web` with routes exported from `admin.rs`
(`ROUTE_ADMIN_FEATURE`/`ROUTE_ADMIN_SPEC`) so router and catalog cannot diverge,
and a new `frontend/` Authoring-actions panel drives preview → confirm → typed
result. `doctor`/`inspect`/`upgrade` stay on the existing workbench routes, and
transport/build/PTY/interactive `agent` commands keep their honest CLI-only
disposition — publish/deploy remain in the delivery controls.

Evidence: `cargo build` clean; `cargo test` 1212 passed, 0 failed; new
`tests/forge_web_command_execution_contract.rs` (5) plus catalog (8) and adjacent
workbench/delivery/portfolio (11) green; `openspec validate --all --strict
--no-interactive` 70 items pass; name preflight and `cargo fmt --check` pass. A
headless-Chromium run against a throwaway temp registry (never the user's real
projects) signed in, previewed `feature add`, confirmed with the exact digest,
showed "Accepted as journaled operation … Core handler", wrote `auth: 0.1.0` to
the temp manifest, produced no console errors, and read the authenticated
`/v1/admin/commands` catalog showing `feature.add` and `spec.generate` as `web`.
Two commits: backend routes + catalog (`377c335`), then frontend panel + archive
+ this handoff. Nothing pushed.

### forge-identity-password-management delivered and archived (2026-10-07)

`forge-identity-password-management` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-identity-password-management`, adding
two `forge-admin-login` requirements. `forge identity change-password` rotates
the single administrator password in place (Argon2id re-hash, email preserved),
refuses weak/mismatched input and a change before any administrator exists, and
revokes every outstanding browser session on success. `forge identity
generate-password [--length]` prints one OS-entropy password (default 20,
accepted 12–128) without touching the registry. New functions
`global::change_password` / `revoke_all_sessions` / `generate_password` reuse the
existing `argon2` and `rand` dependencies — no new crate, no schema change.

Evidence: `cargo test --lib identity::global` (6 passed — 3 new),
`cargo test --test identity_contract` (19 passed), `cargo fmt --check` clean,
`cargo build` 0 errors (2 pre-existing warnings), name preflight PASS,
`openspec validate --all --strict --no-interactive` (70 passed), `git diff
--check` clean. Live round trip against the running API: `generate-password`
emitted a value, `change-password` (PTY) reported success, the old password then
returned `401` and the new one `200` at `POST /v1/admin/session`.

No active changes remain, so this handoff carries no `current_spec` pointer.

`forge-web-delivery-controls` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-delivery-controls`, promoting four
`forge-web-delivery-controls` requirements into a new canonical spec. It delivers
the sharing→approve→publish flow the portfolio package deferred, on the same
session-gated, exact-origin, JSON-only admin boundary as the other `/v1/admin`
routes: share allowlist add/remove, a side-effect-free preview that mints the
current SHA-256 manifest digest, digest-bound approval, publication through the
Core `publish_approved_manifest` chain, reconciliation of ambiguous/unknown
attempts and per-operation status all live behind `GET`/`POST /v1/admin/delivery*`.
Every mutating step is confirm- and digest-bound — an unconfirmed call or a
stale/mismatched digest is refused with no effect and a refreshed preview. Nothing
runs `sh -c`, a generic shell or interpolated argv; the browser never supplies a
filesystem path (the artifact target comes only from server-side
`FORGE_SHARE_PUBLISH_TARGET`, an unset target is a typed `409` prerequisite and no
response serializes an absolute path — the `local-file:<path>` publisher label is
reduced to a safe kind). Publication from the web always dispatches
`adapter: None` (default-safe local export); the subprocess adapter,
repository/provider operations beyond share publication (commit/push/mirror,
release/deploy, GitHub metadata, docs translation and Studio) stay honestly
`cli_only`/`provider_required` in the catalog and are never substituted with shell
execution.

Before that, `forge-web-portfolio-controls` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-portfolio-controls`, promoting
three `forge-web-portfolio-controls` requirements into a new canonical spec. It
adds cross-project portfolio metadata controls and truthful evidence views to the
session-gated, exact-origin admin surface: Forge-owned tags, relations, reviews
and goals are managed through typed in-process Core writes, while imported
source-owned evidence stays read-only and append-only. The cross-project evidence
bundle projects catalog, gap, fleet, inventory, governance, analytics, provider
matrix and readiness with source, freshness and honest per-source status,
performing no provider probe on load (each row `not_run`) and enforcing the
interest cohort threshold. Every projection is scrubbed of absolute filesystem
paths, and nothing runs `sh -c`, a generic shell or interpolated argv. Portfolio
sharing (allowlist, preview and digest-bound approval) and its publication were
owned by the delivery package and are now delivered and archived above.

Before that, `forge-web-project-workbench` is implemented, verified and archived
as `openspec/changes/archive/2026-10-06-forge-web-project-workbench`, promoting
four `forge-web-project-workbench` requirements into a new canonical spec. The
workbench deep-dives a single managed project from the dashboard: session-gated
detail, side-effect-free plan and confirmed apply, where every triggered
workflow goes through the crate's own typed in-process functions or a strict
fixed allowlist of design-scoped command ids — never `sh -c`, a generic shell
or interpolated user-controlled argv — and anything unsafe or not yet web
available renders the catalog's honest disposition instead of a fake execution.

Before that, `forge-web-command-catalog` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-command-catalog`, promoting three
`forge-web-command-catalog` requirements into a new canonical spec. Every
top-level and nested Rust CLI command (225 rows, including `help`) is now
discoverable through the authenticated `GET /v1/admin/commands` JSON endpoint
and a searchable Commands section of the standalone frontend, each mapped to a
truthful availability state (`web`, `cli_only`, `provider_required`,
`project_capability_required`, `not_yet_web`) with plain-language guidance; no
shell/eval route exists anywhere and Clap-tree parity is a hard test failure on
drift.

Before that, `forge-web-project-fleet` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-project-fleet`, promoting three
`forge-web-project-fleet` requirements into a new canonical spec. The dashboard's
`GET /v1/admin/projects` now returns a normalized fleet envelope that combines the
local registry, an explicitly selected portable inventory source and the workspace
fleet observer, and always includes a guaranteed Forge-self record; the standalone
`frontend/` renders the sources and per-project rows with honest
healthy-empty/stale/malformed/unavailable/conflict states and no fake actions.

Before that, `forge-global-admin-portal` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-global-admin-portal`, promoting four
`forge-admin-login` requirements (new canonical spec) and four `portal-web-ui`
requirements.

The delivered portal gives one Forge-wide administrator email/password login and
an all-project dashboard, independent of each project's `forge.yaml` `identity:`
block. Browser HTML/CSS/JS live under `frontend/` and are served by the
standalone Rust `forge web serve` listener; the separate `forge api serve`
listener serves admin auth and the fleet as JSON only. No active change remains
in the `openspec list` queue.

`scaffold-prewires-shared-layer` is implemented, verified and archived. Its
requirements were promoted into
[openspec/specs/scaffold-prewires-shared-layer/spec.md](openspec/specs/scaffold-prewires-shared-layer/spec.md).

At the prior archive checkpoint, no active changes remained. The three portal
packages authored after the queue closed were implemented and archived. The
seven earlier in-flight packages
were consolidated into `runtime-hardening-and-contract-closure` and archived on
2026-10-05 as
`openspec/changes/archive/2026-10-05-runtime-hardening-and-contract-closure`,
promoting 11 requirements and modifying 1 across five canonical specs. At that
archive checkpoint, `openspec list` reported none active and the repository held
118 archived changes and 94 canonical specs; the portal packages below were
authored afterward.

### forge-web-delivery-controls delivered and archived (2026-10-06)

`forge-web-delivery-controls` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-delivery-controls`, promoting four
`forge-web-delivery-controls` requirements into a new canonical spec (4 added, 0
modified). It delivers the share→preview→approve→publish→reconcile→status pipeline
the portfolio-controls package explicitly deferred, behind the same session-gated,
exact-origin, JSON-only `/v1/admin` boundary as every other admin surface:

- `src/api/delivery.rs` (new, private, JSON-only, no markup): eight typed,
  in-process routes — `GET /v1/admin/delivery` (one honest read: allowlist records,
  current manifest preview with its SHA-256 digest, newest approval, publication
  trail, any unreconciled attempt, server-side target state, non-live provider
  matrix with `live_probes: false`, and a self-describing `routes` block),
  `GET /v1/admin/delivery/preview` (side-effect-free plan, `effect: none`),
  `POST /v1/admin/delivery/allowlist/{id}` and
  `POST /v1/admin/delivery/allowlist/{id}/remove` (allowlist add/withdraw),
  `POST /v1/admin/delivery/approve` (approve the exact reviewed digest — Core
  re-derives the draft inside its transaction as a second layer, so a record edited
  between preview and approval is refused), `POST /v1/admin/delivery/publish`
  (publish through `publish_approved_manifest`),
  `POST /v1/admin/delivery/reconcile` (resolve an `unknown` attempt) and
  `GET /v1/admin/delivery/operation/{key}` (journal-backed status by key). Every
  mutation reuses ONLY the crate's own typed functions (`Registry` share audit,
  `portfolio::publication::{preview_manifest, publish_approved_manifest}`,
  `validate_operation_key`, `provider::matrix(false)`); nothing spawns a shell,
  `sh -c`, an interpreter, a Git executable or the publish-adapter subprocess, and
  nothing reads a filesystem path from the request.
- Confirm-and-digest boundary (the load-bearing requirement): every mutating route
  runs through `confirmed_digest`, which refuses `confirm != true` with
  `409 delivery-confirm-required` and an empty/mismatched `plan_digest` with
  `400 delivery-invalid` / `409 delivery-plan-stale` — recomputing the current
  manifest hash and, on a mismatch, returning `effect: "none"` plus a refreshed
  preview. Publish additionally orders confirm → non-empty digest → validated
  operation key → configured target (`409 delivery-prerequisite` if
  `FORGE_SHARE_PUBLISH_TARGET` is unset) → existing approval → approved-hash match,
  and always dispatches `adapter: None` (default-safe local export). Retrying the
  same operation key reconciles against the recorded attempt (`already_present:
  true`, bytes unchanged) rather than publishing twice; an unreconciled `unknown`
  attempt blocks a new publish until reconciled with its exact digest (and
  `unknown` is never an accepted reconciliation outcome).
- Path and privacy boundaries: the artifact target comes only from server-side
  `FORGE_SHARE_PUBLISH_TARGET` and is never serialized; `redact_local_paths` exempts
  self-authored `/v1/...` route tokens (no filesystem path begins with `/v1/`) while
  redacting genuine absolute tokens, `scrub` recurses over the whole projection, and
  Core's `local-file:<absolute path>` publisher label is reduced to the safe kind
  `"local-file export"` before it leaves `publication_block_from_report`. Invalid
  ids and operation keys (shell metacharacters, `..`/`%2e` traversal, uppercase) are
  typed `400`/`404` refusals that never echo the input; a withdrawal of an absent
  record reports `removed: false, effect: none` honestly.
- `src/api/mod.rs` + `src/api/admin.rs` + `src/identity/global.rs`: the eight
  `Route::AdminDelivery*` variants are wired into the existing global-session cookie
  gate, exact-origin CORS (deep-path `OPTIONS` preflights included) and the `415`
  non-JSON gate; `session_actor` resolves the audit actor from the validated session
  cookie only (reads `forge_admin.email`, never the password field), falling back to
  a fixed persona label if the identity read fails.
- `src/api/command_catalog.rs`: the nine `portfolio.share.*` leaf rows
  (set/remove/show/list/preview/approve/publish/reconcile/audit) are flipped from
  `not_yet_web` to `web_at(...)` pointing at the seven new `WEB_ROUTE_DELIVERY_*`
  constants added to `IMPLEMENTED_WEB_ROUTES` (enforced by the internal
  `web_rows_only_point_at_implemented_routes` test); commit/push/mirror,
  release/deploy, GitHub metadata, docs and Studio keep their honest
  `cli_only`/`provider_required` disposition and the overview reports
  `repository_operations.web = false` and `adapter.web = false`.
- `frontend/index.html` + `frontend/app.js`: a standalone Delivery section
  (preview/digest, target/provider state, allowlist roster, approval, publication
  history and unreconciled-attempt gate, plus typed confirm-gated action forms and
  an operation-key lookup) rendered via `textContent` only; `initDelivery(projects)`
  runs after `initPortfolio()`. No markup/eval/shell sink and no new file; the
  `forge web serve` allowlist is unchanged.

Evidence at archive (commands run, real output):

| Check | Result |
|---|---|
| `cargo build` | clean (0 errors; only the same pre-existing warnings — none added) |
| `cargo fmt --check` | clean (exit 0) |
| `cargo test --test forge_web_delivery_controls_contract` | **10 passed / 0 failed** (anon `401` with no delivery data leak on GET and `401` on a JSON-content-type POST; hostile origin `403` including deep-path `OPTIONS` `403`/`204`; unconfirmed mutation `409 delivery-confirm-required` with no effect; stale digest `409 delivery-plan-stale` `effect: none` + refreshed preview + no record/approval written; invalid ids/keys (shell metacharacters, traversal) `400 delivery-invalid` never echoed; empty overview honest not-run + `repository_operations.web false` + `adapter.web false` + self-described routes; publish seam — unset target `409 delivery-prerequisite`, wrong digest `409`, confirmed+matching digest `200` writes `site/portfolio-manifest.json` with `publisher: "local-file export"` and the project id present and no path leak, same-key retry `already_present: true` bytes unchanged, status lookup by key; unknown attempt surfaced and only a confirmed, digest-matched, non-`unknown` reconciliation clears it; withdrawal of an absent record `removed: false effect: none`; frontend standalone JSON-only + shell-free region) |
| `cargo test --test forge_admin_api_contract` / `forge_web_fleet_contract` / `forge_web_command_catalog_contract` / `forge_web_project_workbench_contract` | **4 / 11 / 8 / 11 passed, 0 failed** (compatibility intact; the catalog's strict external allow-list now covers the seven delivery routes and nine share rows) |
| `cargo test --test forge_web_portfolio_controls_contract` | **11 passed / 0 failed** (the deferred sharing flow now delivered without regressing portfolio controls) |
| `cargo test --test forge_portal_frontend_contract` / `portal_ui_contract` | **5 / 39 passed, 0 failed** |
| `cargo test --test api_contract` / `identity_contract` | **11 / 19 passed, 0 failed** (no loopback-bind flake this run) |
| `node scripts/check-openspec-change-names.mjs` | PASS (before and after archive) |
| `openspec validate --all --strict --no-interactive` | **69 passed / 0 failed** |
| `git diff --check` | PASS |
| `openspec archive forge-web-delivery-controls --yes` | **11/13 tasks**, warning for the 2 intentionally-deferred tasks honored via `--yes`; `forge-web-delivery-controls: create`, **4 requirements added / 0 modified** into `openspec/specs/forge-web-delivery-controls/spec.md`; **no `--skip-specs`**; archived as `2026-10-06-forge-web-delivery-controls`; `openspec list` subsequently reports **no active changes** |

Honest scope of verification: the live provider/remote write is exercised only at
a deterministic typed seam — a temporary local directory as the write/publish
oracle — so publication success/failure/partial/unknown/reconciliation states are
proven real and honest, but no live interactive-browser session or live-provider
sandbox was run here (task 4.2's live portions are left unticked). Task 2.3
(mirror/GitHub typed browser operations) is intentionally NOT implemented: those
families keep their `provider_required`/`cli_only` disposition and are covered
deterministically at the catalog and typed-refusal seam rather than faked with
shell execution; the delivery is honest that a browser publish confirms the
artifact on this host only, with promotion beyond it staying with the CLI
provider. This change did **not** run `cargo clippy` or a full-workspace
`cargo test`; verification was the focused new contract suite plus the directly
relevant existing suites above (129 tests, 0 failures), run once, per the delivery's
effort budget. Pre-existing conditions from prior entries (untouched clippy lints,
the `artifact_baseline_contract` changelog check, the `Text file busy` test-harness
race) were neither re-run nor altered by this change.

**Program completion:** `forge-web-delivery-controls` was the last active OpenSpec
change in the queue. With it implemented, verified and archived, `openspec list`
reports no active changes and the web command-center program — global admin portal,
project fleet, command catalog, project workbench, portfolio controls and delivery
controls — is complete.

### forge-web-portfolio-controls delivered and archived (2026-10-06)

`forge-web-portfolio-controls` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-portfolio-controls`, promoting three
`forge-web-portfolio-controls` requirements into a new canonical spec (3 added, 0
modified). Cross-project portfolio metadata controls and truthful evidence views
now sit behind the same session-gated, exact-origin admin boundary as the other
`/v1/admin` routes:

- `src/api/portfolio.rs` (new, private, JSON-only, no markup): five typed,
  in-process Core projections — `GET /v1/admin/portfolio` (fleet of Forge-owned
  records with `tag`/`lifecycle`/`confidence` filters),
  `GET /v1/admin/portfolio/evidence` (a cross-project bundle projecting catalog,
  gap report, fleet, portable inventory, governance selection, analytics support,
  a **non-live** provider matrix and readiness — each labelled with source,
  freshness and its real status; no probe runs on load, every provider/readiness
  row is `not_run`), `GET /v1/admin/portfolio/{id}` (per-project detail),
  `GET /v1/admin/portfolio/{id}/evidence` (read-only source-owned snapshots with
  provenance and `effective_status`, `editable: false`) and
  `POST /v1/admin/portfolio/{id}/{action}` where `action` is one of the fixed
  vocabulary `tags`/`relations`/`reviews`/`goals` — each mapping to exactly one
  Forge-owned Core write that records actor and timestamp. The `evidence` action
  is the honest refusal path (`403 portfolio-source-owned`, `effect: none`) that
  leaves the imported snapshot byte-unchanged. Reuses only the crate's own typed
  functions (`Registry` portfolio ops, `catalog::collect`, `doctor::gaps`,
  `fleet::load`, governance constants, `analytics::*`, `provider::matrix(false)`,
  `readiness`, `portfolio::interest`); nothing spawns a shell, reads a path from
  the request, or interpolates argv.
- Path and privacy boundaries: every projected string is recursively scrubbed of
  absolute filesystem-path tokens before it leaves the module, Core `Display`
  text is never echoed, invalid ids (including shell metacharacters, `..`/`%2e`
  traversal, uppercase) are typed `400`/`404` refusals that never echo the input,
  the interest aggregate is withheld (with a named reason) below the configured
  cohort threshold, and stale/unavailable/invalid evidence renders its honest
  state while retaining the other sources.
- `src/api/mod.rs` + `src/api/admin.rs`: the five `Route::AdminPortfolio*`
  variants are wired into the existing global-session cookie gate, exact-origin
  CORS, `415` on non-JSON bodies, and the exhaustive `authorize()`/dispatch
  match — the `evidence` collection segment is matched before the generic `{id}`
  to avoid collision. `mod portfolio;` is a private module.
- `frontend/index.html` + `frontend/app.js`: a standalone Portfolio section
  (project roster, a read-only cross-project evidence view, and accessible tag +
  review controls) rendered via `textContent` only; the only browser writes are
  the typed metadata POSTs. No markup/eval/shell sink and no new file; the
  `forge web serve` allowlist is unchanged.
- Scope reconciliation: the change's spec, proposal and design listed
  share/allowlist/preview/approval among this surface, but the design also assigns
  the sharing→approve-digest→**publish** flow to the delivery package and the spec
  scenarios cover only review recording and refusing source-owned edits. This
  package therefore delivers the mandatory, scenario-backed metadata + evidence
  controls; the sharing/preview/approval record and its publication are owned by
  the delivery package. Requirement #1 of the promoted spec was aligned to that
  delivered scope so the canonical spec does not over-claim; tasks 2.4 and 4.2
  are left unticked to reflect the deferral (archive reported 10/12).

Evidence at archive (commands run, real output):

| Check | Result |
|---|---|
| `cargo build` | clean (0 errors; only the three pre-existing warnings — `ShareSurface` in `src/portfolio/share/validation.rs`, `journal` field / `Published` variant in `src/main.rs` — none added; portfolio code warning-free) |
| `cargo fmt --check` | clean |
| `cargo test --test forge_web_portfolio_controls_contract` | **11 passed / 0 failed** (anon `401` with no data leak on list/evidence/detail/read/tags-write; hostile origin `403` incl. every deep-path `OPTIONS` `403` from an untrusted origin and `204` from the configured origin; shell-metacharacter/cross-path ids `400`/`404` with the input never echoed and no absolute path serialized; unmanaged/observed-only id honest `404 portfolio-unmanaged-project` `effect: none`; empty registry honest-empty with every provider row `not-run`, readiness `not_run`, interest `no_projects`; Forge-owned tag/review/goal mutations recorded and read back in both list and detail with `effect: forge-owned-write` + `actor: global-admin`; source-owned evidence edit `403 portfolio-source-owned` leaving the snapshot byte-unchanged and an unknown read kind `404`; a seeded stale/unavailable/invalid/fresh evidence set renders each honest `effective_status` (with observation time) while retaining all four sources; interest aggregate withheld with a named reason below threshold; per-source states reported independently; frontend standalone + JSON-only, no `eval(`/`new Function`/`child_process`/`innerHTML`/`sh -c` in the portfolio region) |
| `cargo test --test forge_admin_api_contract` / `forge_web_fleet_contract` / `forge_web_command_catalog_contract` / `forge_web-project-workbench_contract` | **4 / 11 / 8 / 11 passed, 0 failed** (compatibility intact) |
| `cargo test --test forge_portal_frontend_contract` / `portal_ui_contract` | **5 / 39 passed, 0 failed** |
| `cargo test --test api_contract` / `identity_contract` | **11 / 19 passed, 0 failed** (no loopback-bind flake this run) |
| `node scripts/check-openspec-change-names.mjs` | PASS (before and after archive) |
| `openspec validate --all --strict --no-interactive` | **69 passed / 0 failed** (before and after archive) |
| `git diff --check` | PASS (after removing the single trailing blank line `openspec` wrote into the promoted spec) |
| `openspec archive forge-web-portfolio-controls --yes` | **10/12 tasks**, warning for the 2 intentionally-deferred tasks honored via `--yes`; `forge-web-portfolio-controls: create`, **3 requirements added / 0 modified** into `openspec/specs/forge-web-portfolio-controls/spec.md`; **no `--skip-specs`**; archived as `2026-10-06-forge-web-portfolio-controls` |

Honest scope of verification: the portfolio DOM roster, evidence rendering and the
tag/review controls are pinned by the standalone frontend source contract and the
typed admin-route contract tests at the HTTP/JSON level, but were **not** rendered
in an interactive headless browser here — the browser-coverage task (4.2) is left
unticked. No live provider/readiness probe is performed or wired in this package
(every row is `not_run` on load); the explicit live opt-in and the
share/preview/approval/publish record belong to the delivery controls package
(task 2.4 left unticked). This change did **not** run `cargo clippy` (it was not
in the mandated verification set) nor a full-workspace `cargo test`; verification
was the focused new contract suite plus the directly relevant existing suites above,
per the delivery's effort budget. The security file-scan hook repeatedly flagged
`eval(`, `sh -c` and `innerHTML` in the contract test — these appear only as
literal forbidden-substring assertions that prove the frontend is XSS/shell-free and
are never executed; recorded here as a deliberate false positive. Pre-existing
conditions from prior entries (untouched clippy lints, the
`artifact_baseline_contract` changelog check, the `Text file busy` test-harness
race) were neither re-run nor altered by this change.



`forge-web-command-catalog` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-command-catalog`, promoting three
`forge-web-command-catalog` requirements into a new canonical spec (3 added, 0
modified). The complete CLI surface is now browser-discoverable as static
metadata:

- `src/api/command_catalog.rs` (new): typed rows
  `{id,parent_id,label,summary,category,scope,risk,availability,route,cli_invocation,reason,capabilities}`
  — 225 rows, one per Clap path (47 top-level + 138 second-level + 39
  third-level) plus the explicit top-level `help` row clap only generates at
  build time. IDs are Clap dot paths (`project.github.observe`); categories
  are the nine derived groups; availability/risk/scope are closed vocabularies
  enforced by `problems()`. Contract is versioned separately as
  `forge-command-catalog/0.1.0`. Web availability is evidence-based: exactly
  four rows (`list`, `fleet.list`, `fleet.status`, `inventory.show`) point at
  the implemented `GET /v1/admin/projects`; every other row carries a
  plain-language reason and next step (transports, native build/test, hidden
  TTY input, local git, provider credentials and manual OIDC callbacks stay
  `cli_only`/`provider_required`/`project_capability_required`; planned web
  workflows are honestly `not_yet_web` tracked gaps, never "supported").
  `disabled` is kept in the vocabulary but unused today.
- `src/api/mod.rs` + `src/api/admin.rs`: `Route::AdminCommands` behind the
  same global-session cookie gate and exact-origin CORS as the other admin
  routes; `GET /v1/admin/commands` returns the envelope or 401/403/503 JSON.
  POST → 405 via the existing alt-method rule. No shell/eval route exists or
  is added anywhere; the catalog is descriptive and authorization stays with
  each invoked operation.
- `src/main.rs`: parity oracle `command_catalog_tests` walks the real
  `Cli::command()` tree (plus `help`) and fails if the catalog and the CLI
  ever drift — the enumeration cannot go stale silently.
- `frontend/index.html`, `app.js`, `styles.css`: a Commands section with
  literal search (search terms only — never shell input), category and state
  filters, availability/risk badges rendered via `textContent`, the CLI
  invocation shown as inert `<code>` guidance, an empty-result state and an
  honest "Command catalog unavailable" state. No new files; the `forge web
  serve` allowlist is unchanged.

Evidence at archive (commands run, real output):

| Check | Result |
|---|---|
| `cargo build` | clean (0 errors; only the same three pre-existing warnings — none added) |
| `cargo fmt --check` | clean |
| `cargo test --bin forge command_catalog_tests` | **2 passed / 0 failed** (full Clap-tree set equality; `problems()` empty) |
| `cargo test --lib api::command_catalog` | **4 passed / 0 failed** (integrity, 225-row count, web-route allowlist, id/parent shape) |
| `cargo test --test forge_web_command_catalog_contract` | **8 passed / 0 failed** (anon 401 without data leak; hostile origin 403 with valid session; authenticated 200 envelope: unique ids, valid parents, reason on every non-web row, vocabulary closure, all 48 top-level names incl. `help`, nested spot checks, exactly 4 web rows; POST → 405; seven exec/shell/run-style paths → 404; shell-metacharacter query never echoed or applied; real-binary `forge --help`/`forge project --help`/`forge portfolio --help` name parity; frontend source contract: catalog fetch, textContent labels, no `eval(`, no markup in Rust) |
| `cargo test --lib` (full) | **1204 passed / 0 failed / 1 ignored** (`generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain` ran and passed here, not skipped) |
| `cargo test --test forge_admin_api_contract` / `forge_web_fleet_contract` / `identity_contract` / `portal_ui_contract` / `forge_portal_frontend_contract` | **4 / 11 / 19 / 39 / 5 passed, 0 failed** (compatibility intact) |
| `cargo test --test api_contract` | **11 passed / 0 failed** on rerun (first run: 10/11 — `healthz` failed to bind an ephemeral loopback port, the documented sandbox flake, not a regression) |
| live loopback round-trip | real `api serve` (18777) + `web serve` (18778) with a PTY-seeded admin: anonymous `GET /v1/admin/commands` → `401`; login → `200`; catalog → `200` with **225 rows / 9 categories / 4 web rows / reason on every non-web row**; POST → `405`; `/v1/admin/exec` → `404`; `/index.html` → `200`; smoke servers stopped afterwards; the pre-existing `api serve` on 8765 was left untouched |
| `node scripts/check-openspec-change-names.mjs` | PASS (before and after archive) |
| `openspec validate --all --strict --no-interactive` | **69 passed / 0 failed** (before and after archive) |
| `git diff --check` | PASS |

Honest scope of verification: catalog DOM rendering, filtering and the empty
state are pinned by the frontend source contract and the live HTTP smoke, but
were **not** exercised in an interactive headless browser here; that visual
pass remains for the operator. Pre-existing conditions re-captured during this
delivery (none added by it): `cargo clippy --all-targets -- -D warnings` still
fails only on untouched files — `src/gate/evidence.rs` (5),
`src/publish/fleet.rs` (2), `src/publish/mod.rs` (2), `src/api/ui/auth.rs` (2),
`src/portfolio/share/validation.rs` (1) and `src/api/fleet.rs` (2) — with
`command_catalog.rs` and its contract tests clippy-clean;
`tests/artifact_baseline_contract.rs::reported_manifest_changelog_versions_agree`
still fails pre-existing (5 passed / 1 failed). Per-user role filtering is
deferred (single global administrator).

### forge-web-project-fleet delivered and archived (2026-10-06)

`forge-web-project-fleet` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-project-fleet`, promoting three
`forge-web-project-fleet` requirements into a new canonical spec (3 added, 0
modified). The dashboard's authenticated JSON endpoint now composes one normalized
fleet from several sources and always includes Forge itself:

- `src/api/fleet.rs` (new, private, JSON-only, no markup): resolves the local
  registry, an explicitly selected portable inventory source and the workspace
  fleet observer through bounded read-only adapters, then merges them with
  provenance. A guaranteed Forge-self record is always present at index 0; a
  registry row carrying the self identity is merged into that single record
  (`has_registry_ref`) rather than duplicated, so Forge appears exactly once.
  Each row keeps its legacy keys (`id/profile/state/lifecycle/confidence/tags/
  evidence/updated_at`) plus normalized fields, and mutates only Forge-managed
  rows (capability labels). Cross-source identity collisions are retained and
  conflict-marked with capabilities cleared — never silently dropped.
- Provenance is exposed per source as a `SourceDescriptor` (`id/kind/status/
  count/malformed/reason/observed_at/provider`) that never carries an absolute
  filesystem path. Adapter failures emit a fixed safe reason; the path-bearing
  error text and `report.source`/`snapshot.source` are never serialized. Healthy-
  empty, fresh, stale, unconfigured, unavailable, malformed (named, not dropped)
  and conflict states are all preserved.
- `src/api/admin.rs`: `GET /v1/admin/projects` delegates to `fleet::load(db)` after
  the session check, returning the envelope or a neutral unavailable response;
  auth, CORS and the no-markup portal contract are unchanged. `src/api/mod.rs`
  adds `mod fleet;`.
- `frontend/index.html`, `app.js`, `styles.css`: a source panel (`role="status"
  aria-live="polite"`) plus a source filter, and a seven-column project table
  (Project/Source/Profile/Latest state/Evidence/Lifecycle/Access). Everything is
  rendered via `textContent` only; capabilities show as labels with no links, so
  there are no fake actions. Deep project navigation is intentionally deferred to
  the future `forge-web-project-workbench`.

Evidence at archive (commands run, real output):

| Check | Result |
|---|---|
| `cargo build` | clean (0 errors; only pre-existing warnings: `ShareSurface` in `src/portfolio/share/validation.rs`, `journal` field / `Published` variant in `src/main.rs`, none added by this change) |
| `cargo fmt --check` | clean |
| `cargo test --lib api::fleet` | **8 passed / 0 failed** (self merge, conflict detection, source classification, summary counts, ordering) |
| `cargo test --test forge_web_fleet_contract` | **11 passed / 0 failed** (anon 401 no data; self on empty registry; self merges registered same-id; managed registry rows distinct; inventory + workspace observed rows; stale inventory; malformed named not dropped; configured-but-unreadable → unavailable safe reason with no path; cross-source conflict retains row and disables links; no absolute path serialized with all sources active) |
| `cargo test --test forge_admin_api_contract` | **4 passed / 0 failed** (empty-registry assertion updated to expect the single self row with `registered:0`, `self_present:true`) |
| `cargo test --test forge_portal_frontend_contract` | **5 passed / 0 failed** |
| `cargo test --test api_contract` / `identity_contract` / `portal_ui_contract` | **11 / 19 / 39 passed, 0 failed** (compatibility intact) |
| live loopback round-trip | real `api serve` (18777) + `web serve` (18778) with the exact frontend origin: `/healthz` 200; anonymous `GET /v1/admin/projects` → `401 api-unauthorized`; hostile origin → `403`; `/index.html` 200; `/app.js` 200 `text/javascript; charset=utf-8`; path traversal → `404` |
| `node scripts/check-openspec-change-names.mjs` | PASS (before and after archive) |
| `openspec validate forge-web-project-fleet --strict` | valid (before archive) |
| `openspec validate --all --strict --no-interactive` | **69 passed / 0 failed** (before and after archive) |
| `git diff --check` | PASS |

Honest scope of verification: the source-panel and project-row DOM rendering and
the search/source-filter re-render are covered by the standalone frontend source
contract and were exercised at the HTTP/JSON level against the running listeners,
but were **not** rendered in an interactive headless browser here; that visual pass
remains for the operator. Source selection stays strictly env-driven
(`FORGE_INVENTORY_SOURCE`, `FORGE_WORKSPACE_REGISTRY`, plus `FORGE_SELF_ID` and
`FORGE_FLEET_MAX_AGE_SECONDS` overrides) — no sibling-directory scanning, no
absolute path leakage, no mutation of unmanaged rows.

### portal-browser-sign-in delivered and archived (2026-10-06)

`portal-browser-sign-in` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-portal-browser-sign-in`, promoting one
`central-admin-identity` requirement and four `portal-web-ui` requirements into
the canonical specs. The browser round trip runs through the `openidconnect`
crate's discovery, PKCE exchange and ID-token verification behind a
`BrowserAuthVerifier` trait (`LibraryBrowserAuthVerifier` in production, a
fake verifier in tests). The pin moved from the drafted `3.5.0` to `4.0.1`
because the 3.x `reqwest` 0.11 / `rustls` 0.21 stack carries open
`rustls-webpki`/`h2` advisories; the unavoidable `rsa` timing advisory is a
reasoned exception in `deny.toml` and `.cargo/audit.toml`.

Evidence at archive: `cargo test --test portal_ui_contract` (31 passed),
`cargo test --test identity_contract` (19 passed), `cargo build`,
`cargo fmt --check`, `cargo deny check` (all four policies ok), `cargo audit`,
`openspec validate --all --strict --no-interactive`,
`node scripts/check-openspec-change-names.mjs` and `git diff --check`.

Pre-existing conditions, not regressions (all present on `HEAD` before this
change, in files it does not touch):

- `cargo clippy --workspace --all-targets --all-features -- -D warnings` fails
  under the host's newer clippy on `src/gate/evidence.rs`,
  `src/publish/fleet.rs`, `src/publish/mod.rs`, `src/api/ui/auth.rs` and
  `src/portfolio/share/validation.rs`. Every file this change touches is clean.
- `tests/artifact_baseline_contract.rs::reported_manifest_changelog_versions_agree`
  fails because `c2ef70b` opened `CHANGELOG.md` with `## [Unreleased]` while the
  test asserts the newest heading equals the `Cargo.toml` version (`0.1.0`). The
  test reads only `CHANGELOG.md` (unmodified) and the `Cargo.toml` version, so the
  outcome is independent of this change. Not repaired here: the fix belongs with
  the changelog convention, not this package.
- Live-provider OIDC acceptance is unavailable in this environment.
  Deterministic coverage sits at the `BrowserAuthVerifier` seam plus the real
  verifier's pre-network refusals (provider error, empty code, expired
  challenge).

### portal-accessible-responsive-ui delivered and archived (2026-10-06)

`portal-accessible-responsive-ui` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-portal-accessible-responsive-ui`,
promoting four `portal-web-ui` requirements. Every portal page now renders one
accessible document shell (`chrome()`): language, skip link, one named
`<header>`/`<nav>`, exactly one `<main id="main-content">`, one page-level
`<h1>` and one `<footer>`. The inline stylesheet is token-driven with a
light/dark preference, reduced-motion support, a visible high-contrast focus
ring, minimum pointer targets and 320px reflow; data tables are captioned,
scoped and wrapped in named keyboard-reachable scroll regions. Only
`src/api/ui/render.rs` changed. No client JavaScript, external asset or new
dependency was added.

Evidence at archive: `cargo test --test portal_ui_contract` (39 passed) and
`cargo test --test portal_browser_a11y` (1 passed) — the latter drives
`tests/browser/portal-a11y-check.mjs` over the shipped page families at
320/375/640/768/1280 CSS px in light and dark and reported
`portal-a11y-check: ok (7 pages, light+dark, 5 viewports, 7 screenshots)`;
`cargo fmt --check`, `cargo build`, `openspec validate --all --strict`,
`node scripts/check-openspec-change-names.mjs` and `git diff --check` all pass.
The full workspace run has no new failure (only the pre-existing
`reported_manifest_changelog_versions_agree` below). Clippy is clean for every
file this change touches; the same pre-existing lints remain elsewhere. The
browser harness requires `node`, the pinned `tests/browser/node_modules`
install and a Chromium engine; when any is absent it reports `UNVERIFIED` and
the markup contract layer remains the always-run evidence.

### portal-login-entry-flow delivered and archived (2026-10-06)

`portal-login-entry-flow` is implemented and archived as
`openspec/changes/archive/2026-10-06-portal-login-entry-flow`, promoting the
unauthenticated portal-entry behavior into `portal-web-ui`. Anonymous HTML
requests to `/ui` now redirect to `/ui/sign-in`; that route renders a required
project-id form and continues through the existing project-scoped OIDC flow.
The form does not enumerate the registry or accept credentials.

Evidence: `cargo test --test portal_ui_contract` (39 passed),
`cargo test --test identity_contract` (19 passed),
`cargo test --test api_contract` (11 passed; run outside the sandbox because
the suite binds ephemeral loopback ports), `cargo fmt --check`, `cargo build`
(0 errors; three pre-existing warnings), `openspec validate --all --strict
--no-interactive` (64 passed), `node scripts/check-openspec-change-names.mjs`,
and `git diff --check`.

### forge-global-admin-portal delivered and archived (2026-10-06)

`forge-global-admin-portal` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-global-admin-portal`, promoting four
`forge-admin-login` requirements into a new canonical spec and four
`portal-web-ui` requirements into the existing one (8 added, 0 modified). One
Forge-wide administrator account and browser session now authenticate the fleet
independently of per-project OIDC:

- `src/identity/global.rs`: Argon2id PHC hashes, opaque 256-bit CSPRNG tokens,
  12-hour bounded sessions persisted only as SHA-256 digests, plus revoke/expiry.
- `src/api/admin.rs` and the `Route::Admin*` routing in `src/api/mod.rs`: JSON
  `GET/POST/DELETE /v1/admin/session` and authenticated `GET /v1/admin/projects`,
  with exact-`frontend_origin` CORS and Origin checks. The session cookie is
  `forge_admin_session` (distinct from project `forge_session`), `HttpOnly;
  SameSite=Lax; Path=/`, `Secure` only for an `https` frontend origin. Existing
  `/v1` bearer and project OIDC behavior is unchanged.
- `src/web.rs` + `forge web serve`: Rust static listener serving only the
  allowlisted `frontend/` assets (no directory traversal, GET/HEAD only).
- `forge identity setup --email`: interactive hidden-password CLI (libc termios),
  refusing non-TTY, weak password (<12 chars), bad email and duplicate setup.
- `frontend/`: standalone `login.html`, `index.html`, `styles.css`, `config.js`,
  `app.js` and a README documenting the separate API and web preview commands. No
  page markup, style or script was added to Rust.

Evidence at archive (commands run, real output):

| Check | Result |
|---|---|
| `cargo build` | clean (0 errors; three pre-existing warnings: `ShareSurface` in `src/portfolio/share/validation.rs`, `journal` field / `Published` variant in `src/main.rs` `FleetEntryOutcome`, none added by this change) |
| `cargo fmt --check` | clean |
| `cargo test --test forge_admin_api_contract` | **4 passed / 0 failed** (login+cookie+fleet+revoke, bad creds/untrusted origin, anonymous+expired+OIDC-cookie isolation, logout-origin-mismatch does not revoke) |
| `cargo test --test forge_portal_frontend_contract` | **5 passed / 0 failed** (standalone assets, login has only email/password and no project-id, dashboard drives JSON API with honest empty/unavailable states, a11y/responsive stylesheet, no markup in Rust) |
| `cargo test --lib identity::global` | **3 passed / 0 failed** (lifecycle, weak/bad-email refusal, expiry) |
| `cargo test --lib web::tests` | **1 passed / 0 failed** (allowlist, root serves login) |
| `cargo test --test identity_contract` / `portal_ui_contract` / `api_contract` | **19 / 39 / 11 passed, 0 failed** (compatibility intact; `api_contract` run outside the sandbox for loopback binds) |
| live loopback round-trip | real `api serve` + `web serve`: configured/unconfigured session JSON, bad login → generic `401`, successful login → `HttpOnly; SameSite=Lax; Max-Age=43200` cookie (no `Secure` on loopback), `GET /v1/admin/projects` returns the real registered `forge-demo-proj` row plus empty/`registered:0` states, DELETE → `Max-Age=0` then `401`, hostile origin → `403`; web server serves `/`,`index.html`,`app.js` and rejects traversal/`POST`/unknown with `404`/`405`; `identity setup` refused on non-TTY stdin and succeeded through a pty without echoing the password |
| `node scripts/check-openspec-change-names.mjs` | PASS (before and after archive) |
| `openspec validate --all --strict --no-interactive` | **69 passed / 0 failed** (before and after archive) |
| `cargo deny check advisories licenses` | **advisories ok, licenses ok** (argon2 0.5.3); only benign unmatched-license-allowance warnings |
| `git diff --check` | PASS |

Honest scope of verification: the browser keyboard-navigation and 320px-reflow
visual experience and the login/dashboard DOM rendering are covered by the
markup/source contract above and were exercised at the HTTP/JSON level against
the running listeners, but were **not** rendered in an interactive headless
browser here; that visual pass remains for the operator. No shared Gate Runtime
is configured; no Gate pass is claimed.

### Fresh clones did not build

`.gitignore` excluded `/templates/`, filed under "local agent runtime state" by
`b85f980` alongside `/.ariadex/`, `/.claude/` and `/.commandcode/`. It is not agent
state: `src/generate/mod.rs:662` embeds `templates/rust-web-main-rs.txt` with
`include_str!`, so the `rust-web` scaffold body could never be committed. Proven by
moving the directory aside and building —
`error: couldn't read src/generate/../../templates/rust-web-main-rs.txt`,
`could not compile forge (lib)`. Every clone since `b85f980` was incomplete; only the
working machine had the file. The rule is removed and the template is tracked.

Local agent state for this CLI (`/.qoder/`) is now ignored on the same terms as the
other agent directories, and the root `driftwatch.toml` is tracked, which is what the
`.gitignore` comment next to `.driftwatch/` already said should happen.

## Seven in-flight changes consolidated, then archived

Seven change packages were implemented, verified and committed on `main`, and none
was archived. Each one recorded `openspec archive` as "deliberately not run: the
other changes are parked" — which is inverted, because archive is the per-change
step that removes a change from flight. The queue had jammed on a blocker in
`contract-parity-gate-real-digests` §4.4 (`check-spec-governance.mjs` failing on the
`scaffold-prewires-shared-layer` spec), and that blocker had already been fixed by
`5019d12` without anyone re-running the check. That check reports PASS today.

The seven are merged into one archived package and their directories are removed.
Their full text is preserved in git from `c138f08` to `1c3e1f5`.

| Absorbed change | Code commit | Concern |
|---|---|---|
| `contract-parity-gate-real-digests` | `c138f08` | vendored contract bytes |
| `manifest-wire-contract-shape` | `619b945` | the emitted manifest's wire shape |
| `governance-adapter-request-write-race` | `21a9566` | a governance adapter that never reads its request |
| `governance-adapter-bounded-process-run` | `5d5f103` | the adapter subprocess can deadlock; the revision lookup had no deadline |
| `studio-preview-contract-port-range` | `5d5f103` | a test port range inside the OS ephemeral window |
| `adapter-request-write-boundary` | `e3a156b` | the same broken-pipe request write at the publish, translate and hermora boundaries |
| `studio-test-port-range` | `f078a4c` | three more hardcoded test port ranges inside the OS ephemeral window |

Merging was necessary, not cosmetic: `src/governance.rs` was edited by three of the
five code commits, and `f078a4c` deleted ~100 lines `5d5f103` had just written into
`tests/studio_preview_contract.rs` to move them into the shared
`tests/support/studio_ports.rs`. Archiving separately would have promoted two
near-duplicate port requirements into `runtime-hardening-and-test-isolation` and
written the broken-pipe rule twice at two scopes. After the merge there is **one
requirement per mechanism**:

- the request-write rule in `governance-provider-contract` only, widened to all four
  boundaries and carrying all four unioned scenarios;
- the Studio test port window in `runtime-hardening-and-test-isolation` only,
  carrying the union of six scenarios;
- the allocator refusal, the bounded adapter run, the bounded revision lookup, the
  parity gate and the manifest wire shape carried verbatim.

`openspec archive` ran with spec promotion, no `--skip-specs`:
**11 requirements added, 1 modified**, across `governance-provider-contract` (3),
`platform-contract-consumption` (5 added, 1 modified), `portfolio-share` (1),
`runtime-hardening-and-test-isolation` (1) and `site-studio-preview-refinement` (1).
Verification re-run at archive time rather than inherited: whole workspace
**2305 passed / 0 failed / 3 ignored**, `scripts/contract-parity.sh` **exit 0**
comparing 13 files and 10 family digests, manifest schema acceptance against the
sibling contract **1 passed**, `cargo fmt --check` clean, clippy exit 0,
`check-openspec-change-names` and `check-spec-governance` PASS.

The narrative sections below still name the seven absorbed packages, because each one
records a distinct mechanism and its measured evidence. They are history, not
work-in-flight: every requirement they describe is now in a canonical spec.

## What governance-adapter-bounded-process-run fixes

Three defects in `run_adapter` / `git_revision`, all recorded as outstanding in
`governance-adapter-request-write-race`'s `design.md` §5 and all reachable from
`forge governance check`.

1. **A large stdout deadlocked.** `MAX_ADAPTER_OUTPUT_BYTES` is 256 KiB, a pipe
   buffer is 64 KiB, and the old code read a pipe only *after* `try_wait`
   reported the child gone. Any adapter writing more than one buffer blocked in
   `write(2)` on every run and was reported `Unavailable` —
   `adapter exceeded timeout of 5000 ms`. Arithmetic, not a race. Both pipes
   are now drained on their own threads while the parent waits, bytes past the
   cap counted and discarded so the cap stays a real memory bound.
2. **`git_revision` had no deadline.** `Command::output()` blocks until the
   child exits, forever, and it runs on *every* check including the local
   provider. It is now bounded by the selected provider's existing `timeout_ms`
   — the constant `validate_config` already constrains — so no new number
   enters the boundary. Measured against the pre-fix expression: `timeout 8` →
   **exit 124**, never returned.
3. **The wait polled every 10 ms.** It is now a blocking `recv_timeout` that
   wakes on a pipe reaching end-of-file or at the deadline.

Two further findings came out of the implementation and are documented rather
than papered over. `/bin/sh -c '…'` forks on this host, so killing a child does
not close a pipe a descendant inherited (**measured**: the pipe stayed open the
full 30 s), which is why the drain threads are detached rather than joined — a
join there took the full 30 s, an unbounded wait in the exact case the bound
exists for. And end-of-file is **not** an exit event, so a waiter that woke
only on end-of-file reported `exceeded timeout` for adapters that had answered
completely (**measured**: 2–4 failures per run, at exact multiples of the 5 s
and 10 s test budgets; 0 failures and 0.02 s after the fix). That one narrow
window — both pipes done, child not yet reaped — is the only place a short
re-check remains, and it has its own deterministic guard.

`21a9566` is not regressed: the `BrokenPipe`-is-not-a-failure rule and the
kill-and-reap on a genuine write failure are carried through untouched, and
both of its guards still pass.

### Verification of these two changes (2026-10-05)

| Check | Result |
|---|---|
| `cargo test --test studio_preview_contract` × 30 | **30/30 passed**, `7 passed; 0 failed` every run |
| `cargo test --test studio_preview_contract -- --test-threads=8` × 10 | **10/10 passed** |
| `cargo test --test governance_contract` × 30 | **30/30 passed** |
| `cargo test --test governance_contract -- --test-threads=8` / `=1` | **10/10** and **10/10** passed |
| `cargo test --lib governance::tests` | 17 passed |
| Full suite, 6 runs with the change **stashed** | 4 of 6 **failed**; run c failed `preview_port_collision_is_refused_without_killing_a_listener` with `left: 56, right: 64` |
| Full suite, 5 runs with the change | 2 clean (`2303 passed / 0 failed`), 3 with failures all drawn from the pre-existing pool |
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets` | exit 0; 12 warnings stashed, 12 applied — none added |
| `git diff --check` | PASS |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | 68 passed / 0 failed |

Guard-by-guard, against the pre-fix mechanism, with the guard reverted rather
than the fix assumed:

| Guard | Against the pre-fix mechanism |
|---|---|
| `an_adapter_that_writes_more_than_one_pipe_buffer_still_answers` | **fails**: `left: Unavailable, right: Pass` |
| `an_adapter_that_writes_past_the_output_cap_is_refused` | **fails**: `Ok(... status: Unavailable, detail: "adapter exceeded timeout of 5000 ms")` |
| `a_revision_lookup_that_never_answers_gives_up_within_its_bound` | the pre-fix `Command::output()` expression, rebuilt and run, **never returned**: `timeout 8` → exit **124** |
| `a_child_that_closed_its_pipes_and_keeps_running_is_not_reported_as_a_timeout` | **fails**: `elapsed 5.004397229s` of a 5 s budget |
| Studio liveness loop, one listener removed | **fails**: `listener on 4100 no longer accepts: Connection refused (os error 111)` |

The first full-suite runs with an early version of the fix failed 2–4 tests per
run at exact multiples of the 5 s/10 s test budgets. That was this change's own
bug — end-of-file is not an exit event — and it is documented in the change's
`design.md` §3.4 rather than quietly dropped.

## Whole-suite flakes: four fixed here, four still open and classified

| Test | Signature | State |
|---|---|---|
| `studio_preview_contract::preview_port_collision_is_refused_without_killing_a_listener` | `left: 56, right: 64` | **fixed** by `studio-preview-contract-port-range` |
| `studio_api_contract::api_owns_the_live_preview_between_start_and_stop` | `studio port unavailable: no free port in range 47100..=47163` | **fixed** by `studio-test-port-range` — mechanism reproduced and proved, see below |
| `studio_cli_contract::preview_start_probe_reaches_ready_and_leaves_no_live_process` | same, for `47300..=47363` | **fixed** by `studio-test-port-range` |
| `delivery_cross_surface::a_successful_preflight_writes_a_journal_row_visible_on_both_transports` | `publish provider 'openpanel': Broken pipe (os error 32)` | **fixed** by `adapter-request-write-boundary`; 30/30 target runs |
| `docs_contract::provider_failure_keeps_prior_derivative_and_redacts_secrets` | `translator stdin write failed: Broken pipe (os error 32)` | **fixed** by `adapter-request-write-boundary`; 30/30 target runs |
| `api_contract::{healthz_route_returns_200_without_authorization, unknown_route_returns_404, wrong_method_returns_405}` | `ConnectionReset (os error 104)` | **pre-existing, reproduced stashed**: 3 failures in 8 target runs — untouched here |
| `mcp_contract::mcp_repeated_isolated_round_trip_is_stable` | two invocations, `observed_at` one second apart | **pre-existing, reproduced stashed** — untouched here |
| `docs_contract::ordering_is_stable_by_project_then_source_whatever_the_selection_order` | — | **pre-existing, reproduced stashed** — untouched here |
| `Text file busy (os error 26)` at **three** lib sites — `portfolio::share::publish::tests::publish_timeout_is_bounded`, `policy::tests::unrecognized_json_documents_never_report`, `gate::tests::real_run_executes_gate_surface_and_records_evidence` | `spawn failed: Text file busy (os error 26)` | **pre-existing, root-caused, not fixable in scope** — see "The Text file busy flake, root-caused" below |

The three `BrokenPipe` boundaries named as "the strongest candidate for the next
change" are now fixed; the rule they share lives in `process::write_request`.

## The `Text file busy` flake, root-caused

It is **not** a gate-runner bug, **not** an environmental quirk, and **not** a
collision between parallel test targets. It is a harness race, and it is
reproducible on demand. Four sites now, and the first three are all in the lib
target: whichever one writes a script happens to be racing whichever spawner.

Measured rate on this machine: 1 failure in 30 `cargo test --lib` runs, 1 in 40,
and 1 on the first `strace` attempt. **0 failures in 200 runs of the single test
in isolation** — which is exactly why the previous worker recorded it as
"passes 5/5 in isolation both ways" and could not place it.

The three sites, all in the lib target, all the same shape: write a script into
a temp directory with `fs::write`, `chmod +x`, then `Command::new(that path)`:

| Site | Fixture |
|---|---|
| `src/portfolio/share/publish.rs:444` | `/tmp/forge-share-timeout-<pid>/wedged.sh` |
| `src/policy/mod.rs` (`executable()`) | `<TempDir>/proj/dw-junk.sh` |
| `src/gate/mod.rs:1072` (`write_fixture_script`) | `<TempDir>/gate-ok.sh` |
| `tests/inventory_contract.rs:281` | `<TempDir>/inventory-adapter.sh`, staged with `fs::copy` |

The syscall trace of a real failure shows the mechanism exactly:

```
2445580 openat("…/proj/dw-junk.sh", O_WRONLY|O_CREAT|O_TRUNC|O_CLOEXEC) = 27
…
2445600 execve("…/proj/dw-junk.sh", [… "--version"]) = -1 ETXTBSY
```

`2445600` is a **forked child**, and so are `2445597`, `2445598`, `2445599` and
`2445603` — other tests' spawns, interleaved. `fs::write` holds the
`O_WRONLY` descriptor across its `open`/`write`/`close`, and Rust's spawn path
is `fork` + `execvp`, so **a child forked inside that window inherits a copy of
the write descriptor**. The child then execs the very file that descriptor is
open for. `O_CLOEXEC` does not save it: the kernel checks `ETXTBSY` while
opening the new executable, which is *before* it closes the close-on-exec
descriptors. The exec of an executable that another live descriptor has open
for writing is refused, which is the ordinary "you cannot overwrite a running
executable" rule.

That accounts for every observation, including the ones that looked like
evidence for other explanations:

- **0/200 in isolation** — one thread, so no second spawn can fork inside the
  write window.
- **A different test each time** — it is whichever script-writer happens to be
  racing whichever spawner.
- **Not a shared artifact** — each fixture lives in its own `TempDir`; what is
  shared is the process's descriptor table, not a file.
- **Not the filesystem** — `/tmp` is tmpfs here, but the rule is inode-wide and
  behaves the same on ext4.
- **The leaked directories** (`/tmp/forge-share-timeout-*` on disk right now)
  are a *consequence*: the panic skips the `remove_dir_all` at the end of the
  test, so a failed run leaves its fixture behind.

**Why it is not fixed here.** Closing the window needs every `spawn` in the
process to be excluded from the write window, which means a lock around *all*
process creation in a 1180-test binary plus every integration-test binary.
One of the three sites (`src/portfolio/share/**`) is explicitly outside this
change's scope, and the other two are lib-crate unit tests whose spawn sites are
product code. It is therefore left **named and owned here rather than left
unowned**: the honest statement is that it is a genuine race with a known
mechanism and no fix that fits one change. Scheduling it means picking one of:

1. a crate-wide spawn/write lock used by every test that stages an executable
   (touches product code, or needs the tests moved behind a helper), or
2. replacing staged-script fixtures with `sh -c '<body>'`, which removes the
   writable executable entirely — not possible where the code under test
   resolves a *path*, which is all three sites.

## What adapter-request-write-boundary fixes

`21a9566` fixed a `BrokenPipe` on the *governance* adapter's request write and
recorded the other three as mechanical. Those three are now fixed by one shared
rule instead of three more copies of it:

```rust
// src/process.rs
pub fn write_request(stdin: &mut impl Write, request: &[u8]) -> std::io::Result<()>
```

`BrokenPipe` is `Ok(())` — the peer closed its input, so the caller goes on to
the child's real exit status and output. Every other error is returned for the
caller to map to its own typed refusal.

| Site | Pre-fix | Post-fix |
|---|---|---|
| `src/publish/providers.rs:529` | bare `write_all` → `PublishInvalid`, **child left running** | shared rule, kill + reap, same message |
| `src/docs/mod.rs:676` | bare `write_all` → translator error | shared rule; its existing kill + reap unchanged |
| `src/delivery/hermora.rs:173` | `let _ = stdin.write_all(&payload)` — **every** error discarded | shared rule, kill + reap, typed `DeliveryUnavailable` |
| `src/governance.rs:898` | already correct (`21a9566`) | its mapper delegates to the shared rule; both landed guards untouched |

Hermora was the odd one: its comment already stated the correct rule ("a broken
pipe here means the adapter exited early") and the code under it threw the
error away, so a genuine `EIO`/`ENOMEM` while writing a request was silently
swallowed and Forge then reported the adapter's own exit status for a request
that was never delivered.

**Why one shared function rather than three more edits.** The decision "`BrokenPipe`
reports on the child, so it is not Forge's failure" is policy, not a detail of
one module, and four copies is how Hermora's `let _ =` was written in the first
place — three sites spelled the rule and the fourth had nothing to copy. The
helper takes a `&mut impl Write`, not a `&mut Child`, precisely so the
*non*-`BrokenPipe` arm stays unit-testable with a `Write` impl that always fails
`PermissionDenied`. That guard is what catches a reintroduced `let _ =`.

Two guards in `src/process.rs::tests`, both deterministic:

| Guard | Against the pre-fix behaviour |
|---|---|
| `a_request_write_to_a_child_that_already_exited_is_not_a_failure` | **fails 1/1**: `Os { code: 32, kind: BrokenPipe }` |
| `a_request_write_failure_that_is_not_a_broken_pipe_is_returned` | passes both ways by design — it pins the *retained* refusal |

### The deadlock check on the three new sites

Asked to check each site for the pipe-buffer deadlock `5d5f103` fixed in
`governance.rs`. **The shape is present at all three, and worse**: stdout is
drained only *after* `try_wait` reports exit, and none of the three caps stdout
at all.

| Site | stdout drained while the child runs? | cap | a child writing > 64 KiB to stdout |
|---|---|---|---|
| `src/publish/providers.rs:589-607` | no (stderr *is*, on its own thread, `:546`) | none | blocks in `write(2)`, killed at `provider_timeout()`, reported `timed out` |
| `src/docs/mod.rs:684-693` | no | none | blocked, killed at the translator timeout |
| `src/delivery/hermora.rs:180-200` | no (stderr is `/dev/null`) | `RESPONSE_BYTES_MAX` applies *after* the wait, so it bounds the retained response, not the read | blocked, killed at `adapter_timeout()` |

**It is a misclassification, not a hang**: each loop has an existing deadline it
returns from, so the wrong verdict is "timed out", not "wedged". It is left
unfixed deliberately — closing it means re-deriving the bounded-run machinery
three more times, including its measured `/bin/sh` fork hazard where drain
threads must be *detached* because joining one took the full 30 s. No test in
this repository produces more than ~10 KiB on these channels, so nothing
attributes a failure to it. Recorded in the change's `design.md` §5.

## What studio-test-port-range fixes

Three more targets hardcoded bases inside this host's ephemeral window
(`/proc/sys/net/ipv4/ip_local_port_range` = `32768 60999`):
`studio_api_contract.rs:193` (`47100`), `studio_cli_contract.rs:505,549`
(`47300`) and `react_web_native_preview.rs:153` (`48200`). One shared helper,
`tests/support/studio_ports.rs`, now serves all four Studio targets including
the one the previous change fixed, so there is one search rather than four.

**Mechanism proved both ways.** With an unrelated python process holding all
256 ports of `45800-45863`, `47100-47163`, `47300-47363` and `48200-48263`:

- with the fix, all four targets pass;
- with the pre-fix bases restored, `studio_api_contract` fails
  `studio port unavailable: no free port in range 47100..=47163` and
  `studio_cli_contract` fails `no free port in range 47300..=47363`.

**A defect this change introduced and then fixed.** Its own first full-suite run
failed, because a `react_web_native_preview` run was executing at the same time
and all four targets preferred the *same* window:

```
studio-start-timeout: profile runner exited before binding the reserved port (port 4100)
```

`cargo test` runs test binaries one at a time; two `cargo test` invocations do
not. Each target now starts its search at its own index, so it prefers `4100`,
`4228`, `4356` or `4484` — rotated, not filtered, so every candidate stays
reachable. Measured with `ss -ltn` while three targets ran: `4228` and `4356` in
use at once, from two different targets. The underlying gap is the allocator's
own bind/drop window, which is a product defect and is recorded as such.

`preview_port_collision_is_refused_without_killing_a_listener` bound a fixed
range `45800–45863`, inside this host's ephemeral window
(`/proc/sys/net/ipv4/ip_local_port_range` = `32768 60999`). One unrelated
outbound connection took one port and the test failed `left: 63, right: 64`.
The test now reads the ephemeral window, considers only candidates lying wholly
outside it, and **keeps the listeners** that verified its range was free, so
nothing can change between choosing the range and occupying it. Measured: with
`4100..4163` held by an unrelated process, the whole target still passes 7/7.

The test also had a second, hidden defect. The "listeners are all still alive"
assertion compared a `Vec` length with the value it was built from and **could
not fail**. It now proves a listener still accepts on every port and that no
port was released. The record states plainly that neither half can prove
*ownership*, because nothing observable from outside a process can.

`manifest-wire-contract-shape` is implemented, verified and archived — promoted on
2026-10-05 as part of `runtime-hardening-and-contract-closure`, together with the six
other packages that had been committed without promotion.

## Verification of adapter-request-write-boundary and studio-test-port-range

Every run of every target, 2026-10-05. **400 consecutive runs, all clean.**

| Check | Result |
|---|---|
| 7 targets × 30 runs at default parallelism | **210/210 clean** |
| `cargo test --lib` × 30 | **30/30** `1180 passed; 0 failed` |
| 8 targets × 10 at `--test-threads=8` | **80/80 clean** |
| 8 targets × 10 at `--test-threads=1` | **80/80 clean** |
| held-range experiment (256 ports of the four old bases held) | 4 targets × 3 runs, **12/12 clean** |
| held-range **negative control** (pre-fix bases restored) | `studio_api_contract` fails `no free port in range 47100..=47163`; `studio_cli_contract` fails `no free port in range 47300..=47363` |
| concurrency control (3 Studio targets at once, 5 rounds) | **15/15 clean** with distinct slots; with every slot forced to `0` the same control failed inside five rounds |
| `FORGE_NATIVE_REACT_WEB_PREVIEW=1 cargo test --test react_web_native_preview` | **6/6 passed**, each a real `npm install`, a real Vite dev server on the run-time base and a Playwright render (`VERIFIED: rendered "hello from native-react-preview"`) |
| new guard vs. pre-fix behaviour | `a_request_write_to_a_child_that_already_exited_is_not_a_failure` **fails 1/1** with `Os { code: 32, kind: BrokenPipe }` |
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets` | exit 0; zero diagnostics in any file either change touches, and the per-target warning set is **identical** stashed vs applied |
| `git diff --check` | PASS |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **70 passed / 0 failed** |

### Whole suite, 6 runs each, with and without the two changes

| Run | with the change | change **stashed** |
|---|---|---|
| 1 | 2305 passed / 0 failed / 3 ignored | 2302 / **1 failed** / 3 — `delivery_cross_surface::a_successful_preflight_writes_a_journal_row_visible_on_both_transports` |
| 2 | 2304 / **1 failed** / 3 — `inventory_show_consumes_external_adapter_executable` | 2303 / 0 failed / 3 |
| 3 | 2304 / **1 failed** / 3 — `flutter_scaffold_tests_pass_without_forge` | 2302 / **1 failed** / 3 — `format_json_does_not_alter_checker_stdout` |
| 4 | 2305 / 0 failed / 3 | 2302 / **1 failed** / 3 — the `delivery_cross_surface` BrokenPipe test again |
| 5 | 2304 / **1 failed** / 3 — `flutter_scaffold_tests_pass_without_forge` | 2302 / **1 failed** / 3 — the `delivery_cross_surface` BrokenPipe test again |
| 6 | 2305 / 0 failed / 3 | 2302 / **1 failed** / 3 — `docs_contract::provider_failure_keeps_prior_derivative_and_redacts_secrets` |

**3 of 6 applied runs clean, against 1 of 6 stashed.** Four of the five stashed
failures are the two `BrokenPipe` tests this work fixes, and neither appears in
any applied run. The two-test difference in the totals is the two new unit
guards.

The host matters for reading these numbers: it is shared, and during this work
another tenant had a load average above 30 with several processes pinned at
100 % CPU. Every number above was measured under that load, not on an idle
machine.

### The two applied failures that remain, and their status

| Test | Status |
|---|---|
| `inventory_show_consumes_external_adapter_executable` — `cannot start inventory adapter /tmp/.tmpcyR0pz/inventory-adapter.sh: Text file busy (os error 26)` | **Pre-existing class, not reproduced stashed.** The failure is a *spawn* failure: `tests/inventory_contract.rs` does `fs::copy` of a fixture script and `src/publish/inventory.rs:382` execs it, which is the write-descriptor race above. Both files are untouched by this work and the failure happens before any request write, so it cannot come from it. **Not observed in the 6 stashed runs, and not reproduced in 240 stashed runs of that target** (120 serial + 120 under 6-way parallel load); it needs the whole-suite's process pressure |
| `flutter_scaffold_tests_pass_without_forge` | **Environmental, not reproduced stashed.** It shells out to `flutter test` → `dart pub get`, which needs the network and the whole machine; it failed 2 of 6 applied runs and 0 of 6 stashed. `tests/generate_contract.rs` is untouched by this work |
| `format_json_does_not_alter_checker_stdout` (stashed run 3 only) | **Pre-existing**, seen only with the change stashed |

## What governance-adapter-request-write-race does

`tests/governance_contract.rs` was flaky: ~50 % failure rate, 1–2 tests failing
per run, varying every time. It was reported as a shared-state problem and it
was **not** one. Two measurements settle it:

```
$ for i in $(seq 1 20); do cargo test --test governance_contract -- --test-threads=1; done
7 runs FAILED (1-3 tests each), 13 runs ok
```

Serialising the target does not fix it, which is what a per-test race does and
what shared state does not. The four tests that ever failed are exactly the
four whose adapter script never reads standard input:

```sh
#!/bin/sh
printf '%s' '{"provider":"external", ...}'
```

Such an adapter exits immediately and closes the read end of its stdin pipe,
so Forge's own request write can lose the race and come back
`Broken pipe (os error 32)`. `run_external_provider` maps that to
`ProviderStatus::Unavailable` and **throws the adapter's answer away** — the
answer was already complete on stdout. Measured detail, captured through the
real `save_provider_selection` + `check_project` path:

```
status=Unavailable detail=Some("cannot write adapter request: Broken pipe (os error 32)")
```

The fix is in `src/governance.rs`, not in the test. `BrokenPipe` is the one
write error that says something about the *adapter* rather than about Forge's
plumbing, so `write_adapter_request` treats it as a completed write and Forge
reads the exit status, stdout and stderr the adapter actually produced. Every
other write error keeps its typed unavailable refusal, and now kills and reaps
the child before returning. No test was ignored, serialised, slept or retried.

The shared-state hypothesis was tested and cleared rather than assumed: no
`set_var`/`remove_var` or `set_current_dir` on this path, no `static` /
`OnceLock`, per-test `TempDir`, pid-keyed temp file names that cannot collide
across distinct directories, no socket, and read-only fixtures.

### Verification (2026-10-05)

- `cargo test --test governance_contract`, **245 consecutive runs** at default
  parallelism across five blocks: **244 passed, 1 failed**, and the final
  contiguous block of 20 was **20/20 passed** (see the open item below). A
  further 20 runs at `--test-threads=8`: 20/20 passed. A further 20 at
  `--test-threads=1`: 20/20 passed.
- Before the fix, for comparison: 13 failures across 8 of 20 default runs, and
  7 failures across 4 of 20 serialised runs.
- The deterministic guard was checked against the **pre-fix** code: with the
  `BrokenPipe` arm removed,
  `a_request_write_to_an_adapter_that_already_exited_is_not_a_failure` fails
  1/1 with `cannot write adapter request: Broken pipe (os error 32)`. It is not
  a vacuous test. The end-to-end guard
  `an_adapter_that_never_reads_its_request_still_answers` is a *weaker*
  instrument — it detected the pre-fix defect in 0/10 whole-target runs,
  because `run_adapter` writes immediately after `spawn()` and the child must
  win a sub-millisecond race. It is kept as the only place in the suite that
  names the scenario as a requirement, not as the deterministic pin. A guard
  that raised the odds with CPU pressure or repetition was rejected: that is the
  nondeterminism this change exists to remove.
- `cargo test --workspace --all-targets --no-fail-fast -- --skip
  generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`,
  `cargo fmt --check`, `cargo clippy --workspace --all-targets` (zero findings
  in either file this change touches), `git diff --check`,
  `node scripts/check-openspec-change-names.mjs`,
  `openspec validate --all --strict --no-interactive` — see the totals in the
  change's `tasks.md` §4.
- No shared Gate Runtime is configured; no Gate pass is claimed.

### Outstanding

- **One unexplained failure in 245 runs.** A single run of the 245 (default
  parallelism) reported `20 passed; 1 failed` and the capture did not record
  which test or why. It was not reproduced in the 20 runs at
  `--test-threads=8`, the 20 at `--test-threads=1`, or 220 further default
  runs, and it is **not** claimed as fixed. It is most likely a different,
  environmental flake — see the studio port note below — but that is a
  hypothesis, not a measurement, and it remains open: the capture never
  recorded which test failed, so nothing links it to anything later measured.
- ~~**Three adjacent defects found while mapping `run_adapter`, not fixed here**~~
  **Since fixed** by `governance-adapter-bounded-process-run` (concurrent pipe
  drain, a `git_revision` deadline, a blocking wait instead of a 10 ms poll).
  These three were **not** the cause of this flake — no adapter in this
  repository produces more than ~10 KiB.
- ~~**`tests/studio_preview_contract.rs::preview_port_collision_is_refused_without_killing_a_listener`
  is a separate, pre-existing flake**~~ **Since fixed** by
  `studio-preview-contract-port-range`, which chooses the range at run time
  outside the host's ephemeral window and proves the occupied listeners survive
  the refusal with an assertion that can fail.

## What manifest-wire-contract-shape does

Forge is the **producer** of the manifest a separate static Hugo site consumes.
Three fields it emitted were rejected by the consumed schema, so the first real
export would have failed the consumer's build. The schema is
`platform-contracts/schemas/public-portfolio-manifest.schema.json`
(`platform.public-portfolio-manifest/1.0.0`, digest
`07a3c47769da8923998273cda602ddffb195f983e3dcf344fb1d86540c6bc986`).

| Field | Was | Now |
|---|---|---|
| `schema_family` | `"public-portfolio-manifest"` | `"platform.public-portfolio-manifest"` |
| `schema_version` | JSON number `1` | string `"1.0.0"` |
| `manifest_revision` | JSON number (`u32`) | string `"rev_<revision>"` |

The internal revision is **unchanged everywhere it is stored**: the
`manifest_revision INTEGER` column, the `i64` approval, audit, publication
report and adapter envelope, and `build_manifest(records, u32)`. Only the
serialized field is a string, produced by one function,
`wire_manifest_revision`, so no call site can invent a second spelling.

The encoding is a pure, injective function of the integer, so an unchanged
catalog still hashes identically. One honest consequence: an approval made
before this change is bound to the old hash, so `publish` now refuses it until
the operator previews and approves again. No stored approval or audit entry was
rewritten — that would forge an approval nobody gave.

## Verification (2026-10-05)

- `cargo build`: clean. One `unused import: ShareSurface` warning in
  `src/portfolio/share/validation.rs`, **verified pre-existing** (present with
  this change stashed).
- `cargo test --workspace --all-targets --no-fail-fast -- --skip
  generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`:
  **2294 passed / 0 failed / 3 ignored**. The skip is the pre-existing hang
  described under "Pre-existing conditions" below. The three ignored tests are
  the pre-existing `contract::tests::parity_walk`, the pre-existing
  `packing_leaves_the_sibling_checkout_byte_identical`, and this change's own
  acceptance test, run explicitly.
- `cargo test --test manifest_wire_contract -- --ignored --nocapture`:
  **1 passed** — `jsonschema` accepted a Forge-produced manifest.
- **Acceptance, consumer's own oracle**: a scratch registry, a real
  `target/debug/forge portfolio share set → preview → approve → publish` cycle,
  and then
  `python3 lileililiwen.github.io/scripts/validate_manifest.py --strict`
  against `/home/paul/code/platform-contracts/schemas/public-portfolio-manifest.schema.json`:
  `manifest OK … (schema 1.0.0, 1 project(s), revision rev_1)`, **exit 0**.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate --all --strict --no-interactive`: 65 passed / 0 failed.
- `cargo fmt --check`: clean (it was already clean on `main` before this
  change; the "five dirty files" note further down is stale).
- `cargo clippy --workspace --all-targets`: exit 0, and **zero findings in any
  file this change touches** (`src/portfolio/share/mod.rs`,
  `src/portfolio/share/manifest.rs`, `tests/portfolio_share_cli_contract.rs`,
  `tests/manifest_wire_contract.rs`). The remaining warnings are all in files
  this change does not own.
- `git diff --check`: PASS.
- No shared Gate Runtime is configured; no Gate pass is claimed.

## Three further contract mismatches found, and not fixed

Found while mapping the document, reproduced, and deliberately left as
non-goals of this change rather than silently widening its scope. All three are
reachable from the command line today, and each produces a published manifest
the consumer rejects.

1. **`visibility`** — the schema's enum is `["public"]`; Forge's `Visibility::ALL`
   also admits `unlisted`. `share set --visibility unlisted` publishes a
   document rejected with `projects/0/visibility: 'unlisted' is not one of
   ['public']`.
2. **`status_evidence`** — the schema is `additionalProperties: false` and
   permits only `observed_at`, `source`, `note`; Forge's `EVIDENCE_KEYS` admits
   `observed_at`, `source_system`, `source_revision`, `state`, `source`. A
   record with `--evidence '{"observed_at":"…","source_system":"…"}'` is
   rejected with `Additional properties are not allowed ('source_system' was
   unexpected)`.
3. **`id` length** — the schema caps `id` at 64 characters.
   `validate_project_id` already pins the schema's *pattern* exactly but has no
   length bound, so a 70-character id registers, publishes and is rejected with
   `projects/0/id: … is too long`.

Fixing these means deciding whether Forge narrows its vocabularies or the
contract widens them. That is a product decision, not a mechanical one, and it
belongs to its own change.

## The mirror gap is closed, but its test still points at the sibling

The schema that was missing from Forge's mirror —
`contracts/schemas/public-portfolio-manifest.schema.json` — **is now present**, and
`scripts/contract-parity.sh` verifies it and the twelve other retained files
byte-for-byte against the resolved `platform-contracts` source (13 files, 10 family
digests, exit 0, re-run 2026-10-05). The reason nothing in this repository could
catch the wire-shape defect no longer holds.

One piece of that change's workaround is still standing, and it is now unnecessary
rather than wrong: `tests/manifest_wire_contract.rs::pinned_schema_path` resolves
only `PLATFORM_CONTRACTS_DIR` or a sibling `platform-contracts/` checkout, never the
vendored mirror, so its decisive test stays `#[ignore]`d and is run explicitly.
Repointing it at `contracts/schemas/` would let the schema check run in the ordinary
suite — it would still need Python with `jsonschema`. That is a test behaviour
change, so it is left as open work for its own package rather than folded into a
documentation refresh.

The work landed on `main`. It was originally committed on a
`feat/scaffold-prewires-shared-layer` branch; `main` was fast-forwarded onto it
and the branch deleted, per owner direction that Forge work goes straight to
`main`. See "Why a branch appeared" below.

## What this change does

- A profile may declare a **versioned shared-layer kit** as a compiled-in
  descriptor resolved offline. `forge new` renders that reference into the
  generated project's own native manifest — the `kit` block in `forge.yaml` —
  with no feed access, no sibling checkout and no network (`src/kit/`,
  `src/profile/mod.rs`, `src/generate/mod.rs`).
- A profile declares a **minimum consumption floor**. Unmet is a typed refusal
  before anything is written; there is no warn-and-continue path. An operator
  can override with `--kit-exception <reason>`, which is recorded visibly in
  `forge.yaml` and the README. A profile with no registered kit records a
  **declared zero** with a `zero_reason`, which is a distinct state from a floor
  failure (`src/kit/floor.rs`).
- The `aspnet-web`, `react-web` and `nextjs-web` scaffolds **pre-wire** the
  shared layer. .NET restores from a **committed, project-relative feed** — a
  `NuGet.config` with `<clear />` and one named source at
  `packages/platform-feed` — so a fresh clone restores with no sibling checkout,
  no environment variable and no secret. The Node profiles get digest-pinned
  vendored tokens under an owned `.platform/` subtree with an ownership receipt
  (`src/kit/feed.rs`, `src/kit/assets.rs`, `kits/`).
- `forge kit` is the one new top-level verb: `pack` regenerates the feed,
  `verify` catches drift, `upgrade` is the single sanctioned way a pinned
  project moves between kit versions.
- The pinned kit id and version are observed additively on the existing project
  row. No new SQLite table (`src/registry/mod.rs`).

## The floor, and the evidence it came from

`6` for `aspnet-web`, `1` for the two Node profiles, `0`-with-reason elsewhere.
Derived from per-package distinct external consumer counts, frozen in
`kit::PLATFORM_PACKAGE_EVIDENCE`; a test fails when a classification disagrees
with the fixture.

- Confirmed (6): `Platform.Core` (7), `Platform.AspNetCore` (6),
  `Platform.Testing` (4), `Platform.RateLimiting` (2), `Platform.Idempotency`
  (2), `Platform.Observability` (2).
- Withheld despite clearing the bar, because they need a store:
  `Platform.Persistence.EfCore` (4), `Platform.Identity.AspNetCore` (3),
  `Platform.Tenant.Lifecycle.AspNetCore` (0).
- Provisional (below the bar, rendered commented, never restoring): everything
  else, each comment naming its own consumer count.

The **restore closure is 9 packages, not 6**. `Platform.Billing.Contracts`,
`Platform.Eventing` and `Platform.Web.Telemetry` arrive as project references of
the confirmed set, so the feed carries all nine. The floor still counts only the
six; the three transitive members are never counted and never directly
referenced.

## Verification (2026-10-03)

- `cargo test --workspace --all-targets --no-fail-fast -- --skip generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`:
  **2291 passed / 0 failed**. The one skipped test is discussed under
  "Pre-existing conditions".
- `cargo test --test kit_contract`: **60 passed / 0 failed**, stable across
  three consecutive parallel runs.
- `cargo clippy --workspace --all-targets`: **zero findings in any file this
  change touches** (`src/kit/*`, `src/generate/mod.rs`, `src/main.rs`,
  `tests/kit_contract.rs`). Three findings my own code introduced during review
  were fixed rather than recorded.
- `rustfmt --check`: clean on every file this change touches.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate --all --strict --no-interactive`: 63 passed / 0 failed.
- `git diff --check`: PASS.
- No shared Gate Runtime is configured; no Gate pass is claimed.

## The sibling is now provably read-only

This was the most serious defect found on review, and it is worth stating
plainly because the code *claimed* the opposite.

The spec says no file in the sibling checkout is written, moved or removed, and
`feed.rs` repeated the claim in its own doc comment. It was false.
`dotnet pack -o <dir>` redirects only the final `.nupkg`; it also wrote restore
assets into `<project>/obj/` and build output into `<project>/bin/`, inside the
checkout. Those paths are gitignored, so `git status` stayed clean and the
mutation was invisible to every check that looks at version control.

Proved by running it: the original invocation rewrote **6 file entries** under
`dotnet-platform-libs/src/*/obj/`.

The fix packs from a **scratch copy** of the sibling — `src/` without any
build-output directory, plus every regular file at the root — so the requirement
is true by construction. Redirecting MSBuild's output roots was implemented
first and **rejected on evidence**: it does not work, because the default
`**/*.cs` glob still reads the sibling's own `obj/`, and on a sibling carrying a
stale `net8.0` output the build compiles two copies of the same generated
assembly attributes and fails.

After the fix, a full pack of all nine packages leaves the sibling
**byte-identical across all 10,051 files** under `src/`.

## Defects found and fixed on review

The package's 25 tasks were all ticked before this review. Four of them were
ticked against something that did not hold up:

1. **The explicit kit upgrade had no operator entry point.** `diff_kit_snapshot`
   and `upgrade_kit_snapshot` existed and were tested, but `forge kit`
   implemented only `pack` and `verify`. The spec scenario is "*when an
   operator runs* the explicit kit upgrade"; its only caller was `#[cfg(test)]`.
   A capability nobody can invoke is not an explicit path. Now `forge kit
   upgrade <path> --to <kit@version>`, review-only by default, applied on
   `--confirm`.
2. **The upgrade did not move the declared pin.** It rewrote the owned files
   and the receipt but not `kit.version` in the project's `forge.yaml` — and
   `forge kit verify <path>` deliberately reads what the project *says* it pins.
   A successful upgrade therefore produced a project that failed its own drift
   gate. Now recorded as a targeted line edit, never a YAML round-trip, reusing
   the generator's own `yaml_scalar` so the escaping cannot drift.
3. **The source-mode refusal was dead code.** `refuse_source_reference` existed
   and `FeedRejection::SourceReference` was never constructed outside a test;
   `validate_feed_value` could not produce it. "Forge SHALL refuse a source-mode
   project reference" was unimplemented while looking implemented. Now
   `find_source_reference` scans every rendered manifest before anything is
   staged. A reference that stays *inside* the generated project is deliberately
   allowed — a solution with its own test project is portable.
4. **Two tests asserted nothing.** The pattern-catalog colour check searched
   for the literal string `#[0-9a-fA-F]{6}`, which no hex colour contains. The
   completeness check asserted `matches!(class, Confirmed | Provisional)` against
   a two-variant enum, so "every package is classified" could never fail. Both
   now parse and compare for real.

A fifth, smaller one: the feed's byte-level digest check was gated on
`feed_dir.starts_with(kits_dir())` with no reporting, so a generated project's
own committed feed was version-checked and the byte check was **silently
skipped** — a green result that never ran the check. `FeedVerificationReport`
now carries `digests_verified`, the digest record is a parameter
(`verify_committed_feed_with_digests`), and the CLI prints a note when the check
did not run.

## Pre-existing conditions (not regressions)

- `cargo fmt --all -- --check` used to fail on five files this change does not
  own. **This is now stale and was re-checked on 2026-10-05: the tree is
  rustfmt-clean on `main` at `b8b0fb5`, so that earlier failure has been fixed
  by intervening work rather than by this change.** The claim below is kept as
  written history: at the time, the parent-commit versions of
  `src/gate/evidence.rs`, `src/github/normalize.rs`,
  `src/portfolio/share/validation.rs` and `src/publish/fleet.rs` were already
  rustfmt-dirty, so that failure was not drift from that change.
- `scripts/release-check.sh` blocks at its first gate, `cargo fmt --check`, for
  the reason above. It never reaches its test, clippy, audit or readiness
  stages. `cargo-deny` and `cargo-audit` are both installed.
- `generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`
  **hangs indefinitely** in this environment, at 0% CPU, blocked on the cargo
  package-cache lock taken by the outer `cargo test`. **Verified pre-existing**:
  it hangs identically with this change stashed, in isolation, and was killed by
  a 240s timeout both times. It is the only test excluded from the run above.

## Verification gaps left open

- `packing_leaves_the_sibling_checkout_byte_identical` is `#[ignore]`d, and the
  reason is measured rather than guessed. A real pack is ~15 seconds of heavy
  parallel CPU; in a whole-suite run that load pushed
  `governance_contract::valid_external_response_is_normalized_and_redacted` past
  its 5-second external-adapter timeout (`left: Unavailable, right: Pass`) and
  turned an unrelated green suite red. The control run — same tree, that test
  skipped — is 2291 passed / 0 failed, and the pack test passes on its own, so
  the assertion is sound and the interference is real. Run it explicitly:

  ```sh
  cargo test --test kit_contract packing_leaves_the_sibling_checkout_byte_identical -- --ignored
  ```

  Last explicit run: **passed**.
- The floor-refusal scenario ("WHEN a floor refusal occurs … the destination
  directory, the staging area and the project registry are byte-identical") is
  proven for a real pre-staging refusal through the CLI, but **no shipped
  profile has an unmet floor**, so that exact trigger is unreachable from the
  command line. The typed refusal itself is covered. Closing it properly needs a
  way to induce an unmet floor end to end, which is a product decision.
- `tests/workspace_metadata_contract.rs::opt_out_is_byte_identical_to_pre_release`
  pins the exact bytes `--no-workspace-metadata` produces, per profile. All six
  digests were re-captured because this change alters what a scaffold contains
  by design. The guard's intent is unchanged and it still fails on future drift.
- `forge new`'s `notes` field was already asserted empty for a fully mapped
  profile. A declared-zero warning is not an omission note, so it moved to a new
  additive `kit_warning` field rather than overloading `notes`.
- `tests/generate_contract.rs::dotnet_scaffold_builds_offline_without_forge`
  asserted that an `aspnet-web` scaffold builds with **no** `PackageReference` at
  all. That was true before and is false now, by design. The test is left intact
  and still asserts a successful `dotnet build`; it now needs a resolvable feed,
  which the default test environment does not provide. The equivalent assertion
  added by this change, `the_generated_dotnet_project_operates_without_forge`,
  reports `unverified` rather than a pass when no feed is configured.

## Why a branch appeared

The work was committed on `feat/scaffold-prewires-shared-layer` rather than
straight onto `main`, contrary to the owner's standing direction.

**At the time, AGENTS.md contained no branch instruction.** Its history then was
three commits (`5a2d272`, `d069d59`, `290832f`), and neither AGENTS.md nor
`.ai-rules/` nor README.md mentioned "branch" or "main" anywhere. So the rule the
owner believed was written down was not in the repository, and the previous agent
was not working from a written instruction to override.

The most likely driver is the agent harness rather than the repository: the
default commit guidance in this environment is "if on the default branch, branch
first", which fires even when a project says nothing. The reflog shows the
branch created at `2420e2c` and the single commit landing on it, with `main`
left at `2420e2c`.

Corrective action taken: `main` was fast-forwarded to the commit, the branch
deleted, and all subsequent work committed on `main`. Nothing was lost — the
branch tip and `main` are the same commit.

**The rule has since been written down.** `AGENTS.md` now carries a "Git workflow"
section: work on `main`, no branch or worktree unless the owner explicitly asks,
never push, stage explicit paths and never `git add -A`. That closes the gap this
section recorded; the rest of it stays as the reason the rule is stated where an
agent will read it before committing.

## Native evidence (real, not claimed)

| Profile | Result |
|---|---|
| `aspnet-web` | `dotnet restore` + `dotnet build` on `net10.0` succeeded, 0 warnings / 0 errors, at a path the project was not generated at, with `NUGET_PLATFORM_FEED` unset and no sibling present — the feed bytes are committed in the project |
| `react-web` | `npm run build` and `npm test` offline succeeded; vendored `node .platform/tokens/verify-tokens.mjs` passed |
| `nextjs-web` | `npm run build` and `npm test` offline succeeded |
| `rust-web` | `cargo build --offline` succeeded |
| `flutter-app` | `flutter analyze` — "No issues found!"; `flutter test` — all tests passed |
| `python-service` | rendered; not built |

The portability proof, which is the oracle for the owner's requirement: the
global NuGet package cache was emptied of all 59 `Platform.*` entries first, a
fresh `aspnet-web` scaffold was rendered, copied to an unrelated path, and
restored and built there with the feed variable removed and no sibling library
present. All nine `Platform.*` libraries resolved at `0.1.0` into the emptied
cache, and `obj/project.assets.json` lists exactly 9. The control experiment —
the same project with its committed `packages/` deleted — fails with
`error NU1301`, naming the relative feed as the missing local source. So the
committed bytes are what supply the packages: not the cache, not a sibling, not
an environment variable.

### web-command-reference-browser delivered and archived (2026-10-10)

`web-command-reference-browser` closes web UI/UX audit gap 1,
implemented, verified and archived as
`openspec/changes/archive/2026-10-10-web-command-reference-browser`,
promoting canonical `portal-web-ui` +2 (no `--skip-specs`). Frontend
only; no API/CLI/router/catalog/registry/journal change:

- New `#command-reference` section on the projects view rendering
  every `GET /v1/admin/commands` row from the already-fetched
  `catalogCommands` (no new endpoint/fetch/dep): search (id/label/
  summary/CLI/reason/category) + availability filter (all six states)
  with a live `role=status` count and honest unavailable/no-matches
  states. Every row shows badge + summary + exact `cli_invocation`;
  non-web rows add the plain-language reason + clipboard Copy button
  (clipboard + select-fallback) and never an executable control; web
  rows link to their existing serving view via `referenceViewForRoute`
  (delivery → `/delivery`, portfolio → `/portfolio`, workspace + the
  three server-rooted creations → `/management`, fleet reads →
  `/projects`, else project-scoped → `/workbench`). No new route, so
  `src/web.rs` and the navigation contract are untouched. All catalog
  strings via `textContent` only; the only browser-side effect is a
  clipboard write. Styles: one additive block at the 12px floor with
  44px control targets, wrapped CLI strings, no new motion.
- New static oracle `tests/web_command_reference_browser_contract.rs`
  (6 tests) pinning section/controls, all-six-state filter, badge +
  reason + exact CLI + copy with no executable control, web-row links
  to the five real paths, single catalog fetch, no shell/eval/
  innerHTML in the new block, and the style floor.

Evidence:

| Check | Result |
|---|---|
| `node --check frontend/app.js`, `cargo fmt --check` | clean |
| `cargo build` | 0 errors; same 3 pre-existing warnings |
| `cargo test --test web_command_reference_browser_contract` (new) | **6 passed / 0 failed** |
| `cargo test --test portal_ui_contract` | **21 passed / 0 failed** |
| `cargo test --test forge_web_navigation_contract` | **9 passed / 0 failed** |
| `cargo test --test forge_web_command_catalog_contract` | **7 passed + 2 pre-existing failures** (cap-coverage gap, byte-identical on the pristine tree via `git stash -u` rerun) |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **96 passed / 0 failed** active; **95 passed / 0 failed** after archive |
| `git diff --check` | clean |
| browser oracle (throwaway API+web, scratch `FORGE_REGISTRY`, real Chromium via `/tmp/cmdref-oracle/cmdref-oracle.mjs`, since removed from repo — never committed) | **VERIFIED**: `Showing 234 of 234 commands`, 234 row nodes; `cli_only` → `Showing 81 of 234`, every row badge `CLI only` + reason + Copy, no extra control, CLI starts `forge `; copy announces `Copied forge workspace to the clipboard.`; search `delivery.status` → 1 row linking `/delivery`; no-matches state renders; **zero JS console/page errors** (run with `FORGE_ADMIN_PROJECTS_ROOT=/tmp`) |
| oracle side findings (attribution) | 390px overflow 122px and one `409 /v1/admin/workspace/candidates` (rootless discovery) are **byte-identical on the pristine tree** (probe rerun stashed); the reference section is not among the overflowing elements; with a projects root the error count is zero. Pre-existing, 0 attributable |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (397/397 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| `openspec archive web-command-reference-browser --yes` | archived as `2026-10-10-web-command-reference-browser`, no `--skip-specs`; canonical `portal-web-ui` +2; post-archive validate **95 passed / 0 failed**, names PASS |

Commits on `main`: implementation+specs+frontend+tests (`3f29c50`)
+ this handoff (commit 2); nothing pushed; `openspec list`
reports no active changes and no `current_spec` pointer remains.

### web-project-catalog-browser delivered and archived (2026-10-10)

`web-project-catalog-browser` closes web UI/UX audit gap 3,
implemented, verified and archived as
`openspec/changes/archive/2026-10-10-web-project-catalog-browser`,
promoting canonical `web-project-catalog-browser` +3 (no
`--skip-specs`):

- **Backend (6 typed read-only admin GETs, Core reuse only).** New
  `src/api/catalog_browser.rs` behind `guarded()` + exact-origin
  checks: `GET /v1/admin/projects/catalog` (paginated list, same bytes
  as `GET /v1/projects/catalog`), `GET /v1/admin/projects/{id}/catalog`
  (all records for one id), `.../catalog/tags` and `.../languages`
  (distinct values with counts, CLI `project tags|languages --format
  json` shapes), `.../catalog/gaps` (`?project=` + repeatable
  `?category=`/`?status=`/`?remediation-class=`, CLI `project gaps
  --format json` shape), `GET /v1/admin/fleet/{entry}` (CLI `fleet
  inspect --format json` shape, registry resolves server-side via
  `resolve_registry_path`, browser sends no path). Reuses
  `parse_catalog_query_params` / `build_catalog_selection` /
  `catalog_filter_pairs_from`, `CatalogQuery::from_pairs`, `collect` +
  `apply` / `inspect_records` / `tag_counts` / `language_counts`,
  `gaps::build_report` + `GapFilters`, `fleet::observe` +
  `inspect_entry`. Unknown id → typed `unknown-project`; unknown
  filter/category/status/class → typed `catalog-invalid`; no session →
  401; hostile id → 400 without echo. Registry opened read-only; no
  journal row, no provider, no shell. Router literals precede `{id}`;
  new `fleet` OPTIONS arm; permission `None` (any admin session).
  `IMPLEMENTED_WEB_ROUTES` +6; the six `NotYetWeb` rows
  (`fleet.inspect`, `project.list|inspect|tags|languages|gaps`) become
  `web_at(Read, route, registry_read)` — count stays 234
  (conversion, not addition). `fleet online`, `project github *`,
  `inventory show` untouched.
- **Frontend (gap-1 reference pattern).** New `#catalog-browser`
  section on the projects view after `#command-reference`, before
  `#workbench` (no `src/web.rs`/sidebar/route change): project select
  + inspect rendering every source record with provenance
  (`source_kind`, `source_revision`, `observed_at`, `freshness`);
  tag/language tables with counts; gaps list (verdict badge, category,
  remediation class, `Evidence (source revision · observed_at)` +
  `Showing N of M gaps` / `No gaps — clean`); fleet-entry inspect.
  `textContent`/`el()` only, `role=status` results, error-summary
  focus, 12px floor, no motion, no new dependency. Boot wires
  `initCatalogBrowser()` once; catalog auto-loads tags/languages/gaps.
- **Tests.** New `tests/web_project_catalog_browser_contract.rs`
  (**10 passed / 0 failed**): 4 static (section placement/controls,
  provenance/counts/gaps/evidence tokens + six route strings,
  no-POST/no-shell/no-innerHTML block bound, style floor) + 6 live
  in-process (list byte-parity + provenance, inspect all-source +
  unknown/hostile refusals, tags/languages counts incl. `rust`/`python`,
  gaps evidence + `project+status` scoping + bogus-category 400,
  anonymous 401 + invalid filter + unconfigured-fleet honest refusal,
  registry-bytes-identical read-only proof). Pin updates for this
  change's rows: `tests/forge_web_command_catalog_contract.rs`
  route-allowlist + web-id set, `src/api/command_catalog/catalog.rs`
  web-row vec.
- **Pin repair note.** The web-id-set pin was already stale on the
  pristine tree (byte-identical `git stash -u` rerun): it missed the
  8 `web-lifecycle-execution` rows (`graduation.*`, `intent.*`,
  `remediate.*`, `studio.*`). This change refreshes the pin to the
  current truth (8 lifecycle + 6 catalog-browser rows) so the suite
  distinguishes this change's rows; the remaining `cap`-coverage
  failure is untouched pre-existing.

Evidence:

| Check | Result |
|---|---|
| `cargo fmt --check`, `node --check frontend/app.js` | clean |
| `cargo build` | 0 errors; same 3 pre-existing warnings (`ShareSurface` unused import, `FleetEntryOutcome::Published`, `FLEET_DEFAULT_JOBS`) — one transient `CONTRACT_VERSION` warning introduced mid-change and removed before closeout |
| `cargo test --test web_project_catalog_browser_contract` (new) | **10 passed / 0 failed** |
| `cargo test --lib api::command_catalog` | **7 passed / 0 failed** (incl. updated web-row vec pin) |
| `cargo test --lib contract` | **26 passed / 0 failed** |
| `cargo test --test catalog_contract` / `catalog_cross_surface` / `project_gaps_contract` / `project_gaps_cross_surface` / `fleet_contract` | **18 / 9 / 16 / 18 / 14 passed / 0 failed** |
| `cargo test --test portal_ui_contract` / `forge_web_navigation_contract` / `web_command_reference_browser_contract` / `forge_web_maintainer_surface_contract` / `forge_admin_api_contract` / `api_contract` / `web_login_credentials_contract` | **21 / 9 / 6 / 5 / 11 / 4 / 6 passed / 0 failed** |
| `cargo test --test forge_web_command_catalog_contract` | **8 passed + 1 pre-existing failure** (`cap`-coverage gap, byte-identical on the pristine tree via `git stash -u` rerun; route-allowlist + web-id pins updated and green, incl. repairing the stale lifecycle rows) |
| `cargo test --test project_query_surface_contract` | **11 passed + 2 pre-existing failures** (403-vs-401 origin check, byte-identical on the pristine tree via `git stash -u` rerun) |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **97 passed / 0 failed** active; **97 passed / 0 failed** after archive |
| `git diff --check` | clean |
| live oracle (throwaway API:18766+web:14173, scratch `FORGE_REGISTRY` with alpha/beta fixtures, `FORGE_WORKSPACE_REGISTRY` projects.json with gamma, `FORGE_ADMIN_PROJECTS_ROOT` set, real Chromium via bundled Playwright, script in `/tmp` — never committed) | **VERIFIED**: login → `/projects` → browser ready; inspect alpha renders `local` provenance (`Observed at`, `Source revision`); tag rows `1` (`auth` after registering a feature-bearing manifest); languages `rust`+`python`; gaps `Showing 14 of 14 gaps` with `Evidence (` rows; fleet `gamma` renders; curl cross-check of all six routes 200 + anonymous 401; **zero JS console/page/network errors** |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (399/399 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| `openspec archive web-project-catalog-browser --yes` | archived as `2026-10-10-web-project-catalog-browser`, no `--skip-specs`; canonical `web-project-catalog-browser` +3; post-archive validate **97 passed / 0 failed**, names PASS |

No active changes remain, so this handoff carries no `current_spec` pointer.

### web-shipping-provider-reads delivered and archived (2026-10-10)

`web-shipping-provider-reads` closes audit gap 8: the five
local-config/registry reads that were `not_yet_web` but need no provider
probing — `publish provider list|inspect`, `provider matrix|inspect`,
`plugins list` — are now read-only session-gated admin GETs reusing the
existing Core stores (`src/api/admin/shipping_reads.rs` on the shared
`Route::AdminCreation` triple: `shipping-providers`,
`evidence-providers`, `shipping-plugins`; `router.rs` constant at
999/1000, `deploy.rs` untouched). `fleet inspect` was already web and is
reused as-is with a pointer from the new SPA card. Everything else stays
as-is: `publish sync|db|fleet|prepare|deploy|all`, `docs translate`,
`provider run`, `fleet online`, `project.github *`, `push`/`mirror`,
`readiness *`, `deploy observe`, transports, agent/identity lifecycle
writes, and `publish provider enable|disable` gained no route here.
Secrets/redactions preserved; live-probe verbs untouched.

Implementation commit: `95c964d`. Nothing pushed. No `current_spec`
pointer remains.

Evidence:

| Check | Result |
|---|---|
| `cargo fmt --check`, `node --check frontend/app.js` | clean for all touched files; one pre-existing rustfmt drift in untouched `tests/web_assurance_browser_contract.rs` (reverted, not mine) |
| `cargo build` | 0 errors; same 3 pre-existing warnings (`ShareSurface` unused import, `FleetEntryOutcome::Published`, `FLEET_DEFAULT_JOBS`) |
| `cargo test --test web_shipping_provider_reads_contract` (new) | **10 passed / 0 failed** |
| `cargo test --lib api::command_catalog` | **7 passed / 0 failed** (count stays 234, `problems()` empty, 5-row web vec pin) |
| `cargo test --lib` | **1213 passed / 0 failed** on rerun (first run: 1212 passed + 1 transient failure, name not retained, green on immediate rerun) |
| `cargo test --test portal_ui_contract` / `forge_web_navigation_contract` / `web_command_reference_browser_contract` / `forge_web_maintainer_surface_contract` / `web_assurance_browser_contract` / `web_creation_catalog_browser_contract` / `web_release_deploy_history_contract` / `web_agent_identity_readonly_contract` / `web_project_catalog_browser_contract` / `api_contract` / `forge_admin_api_contract` | **21 / 9 / 6 / 5 / 12 / 10 / 9 / 10 / 10 / 11 / 4 passed / 0 failed** |
| `cargo test --test forge_web_command_catalog_contract` | **8 passed + 1 pre-existing failure** (`cap`-coverage gap, byte-identical on the pristine tree via `git stash -u` rerun; route-allowlist + web-id pins updated and green) |
| `cargo test --test forge_web_project_workbench_contract` | **10 passed + 1 pre-existing failure** (`innerHTML` in login/caret code, byte-identical on the pristine tree via `git stash -u` rerun) |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **102 passed / 0 failed** active; **102 passed / 0 failed** after archive |
| `node scripts/check-spec-governance.mjs` | PASS (after archiver-TBD Purpose repair + pointer removal) |
| `git diff --check` | clean |
| live oracle (throwaway API:18766+web:14173 with rewritten `config.js` copy under `/tmp`, scratch `FORGE_REGISTRY` with alpha/beta fixtures incl. `.forge/providers.yaml`, `FORGE_ADMIN_PROJECTS_ROOT` + `FORGE_FRONTEND_ORIGIN` set, real Chromium via bundled Playwright, script in `/tmp` — never committed) | **VERIFIED**: login → `/delivery` → five reads render (providers list+inspect `jenkins-mac`, matrix all `not-run` incl. `driftwatch-policy`, evidence inspect, plugins, CLI-only remainder with `provider run` + fleet-inspect pointer); curl cross-check of all five routes 200 + anonymous 401; **zero JS console/page/network errors** |
| `forge gate --dry-run` | plan rendered; 9 required checks |
| `forge gate --timeout-secs 600` | **blocked, 0 attributable** — pass: build, governance-quality, placeholder-threshold, product-code-boundary, repository, security, source-file-size (404/404 ≤1000); unresolved pre-existing: declared-verification + tests (`project-runtime` adapter exceeds its fixed 60s budget, identical signature to prior entries). Remediation: warm/point the adapter at the shared target cache or raise the per-check budget, then re-run. This change adds no new failure. |
| `openspec archive web-shipping-provider-reads --yes` | archived as `2026-10-10-web-shipping-provider-reads`, no `--skip-specs`; canonical `web-shipping-provider-reads` +2; archiver-stamped TBD Purpose repaired source-backed before commit; post-archive validate **102 passed / 0 failed**, names PASS, governance PASS |

No active changes remain, so this handoff carries no `current_spec` pointer.
