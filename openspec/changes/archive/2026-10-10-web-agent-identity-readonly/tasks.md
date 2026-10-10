# Tasks: web-agent-identity-readonly

## 1. BFS — Baseline and impact coverage

- [x] Confirm proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/api/admin/history.rs` (matcher + handler precedent), `src/api/admin/deploy.rs` (dispatch + `deploy_id_gate` + `guarded`/`cors`), `src/api/router.rs` (pre-check + OPTIONS + permission/exhaustiveness lists; note 991/1000 cap), `src/api/model.rs` (`Route` docs), `src/api/command_catalog/routes.rs` + `rows_platform.rs` (six `NotYetWeb` leaves + `cli_only` reasons, `IMPLEMENTED_WEB_ROUTES`, 234 count pin), `src/agent/sessions.rs` (`list_sessions`/`status_for`/`probe_provider`), `src/identity/protocol.rs` (`list_sessions`/`load_session`/`lookup_session_in_sibling_projects`), `src/identity/model.rs` (`IdentityConfig::from_manifest_opt`), `src/cli/agent.rs` + `src/cli/identity.rs` (read shapes to mirror), `frontend/index.html` (Workbench grid) + `frontend/app.js` (history-card read pattern), and canonical `agent-runtime-workflows` + `central-admin-identity` + `portal-web-ui` specs.
- [x] Map each requirement/scenario to handler, route arm, catalog row, card control and contract-test assertion; record toolchain and ownership (extend `src/api` + catalog + Workbench card; Core/CLI/schema unchanged).

## 2. DFS — Requirement-by-requirement implementation

- [x] `src/api/admin/routes.rs` + new `src/api/admin/agent_identity.rs`: five route constants + matcher + five session-gated GET handlers reusing `deploy_id_gate`, `guarded`, `cors`, `error`, `scrub_*`; agent kebab + identity hex id gates; path-free projections (`project_path` omitted); `live: null` + CLI-only note (never `live_runtime_status`); spawn-free `probe_provider` availability; identity config read with unconfigured/invalid honesty; every response carries `contract: API_CONTRACT_VERSION`. No write, no provider, no adapter subprocess, no shell, no secret material.
- [x] `src/api/model.rs` + `router.rs` + `admin/deploy.rs`: three `Route` variants, matcher chain, OPTIONS widened in place, permission + dispatch + exhaustiveness threading with `router.rs` verified ≤1000 lines; no existing arm shadowed.
- [x] `src/api/command_catalog/`: five leaves `NotYetWeb` → `web_at` + `IMPLEMENTED_WEB_ROUTES` entries; `identity.session-terminate` → `cli_only`; count pin stays 234; `identity.build-challenge` untouched with its documented reason.
- [x] `frontend/index.html` + `frontend/app.js`: Workbench "Agent & identity" card reusing `#workbench-project` — agent list/inspect, identity sessions list/inspect, identity config read; `role=status` results, error-summary focus, native controls, no new dependency, no `innerHTML`, no inline script.
- [x] `tests/web_agent_identity_readonly_contract.rs`: design §8 oracle (auth, gates, lists, inspects incl. cross-project, config, scrub, no-write-on-read, catalog + frontend pins).
- [x] Delta spec scenarios implemented alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Existing agent/identity CLI behavior unchanged; refused web reads create no state dirs, challenge files or journal rows and spawn no subprocess.
- [x] No registry/journal schema change; no Core/CLI migration.
- [x] Full forbidden-sink + inline-script + path-leak + secret-leak sweep over the new surface (Rust + frontend).
- [x] No current-change placeholder/disabled behavior remains; `IMPLEMENTED_WEB_ROUTES` agrees with all pins; old-frontend safety (unknown ids still typed refusals).
- [x] New/touched Rust files stay under the source-file-size cap (`router.rs` ≤1000 verified).

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (only pre-existing warnings).
- [x] `cargo test --test web_agent_identity_readonly_contract` green; regression: lib, `forge_web_command_catalog_contract`, `portal_ui_contract`, `forge_web_project_workbench_contract`. No new failure (pre-existing failures recorded with pristine-tree evidence + remediation).
- [x] `node scripts/check-openspec-change-names.mjs` PASS; `openspec validate --all --strict --no-interactive` 0 failures; `node scripts/check-spec-governance.mjs` PASS; `git diff --check` clean.
- [x] Live throwaway-registry browser/curl proof: login → Workbench card exercises lists/inspects/config with zero attributable JS errors.
- [x] `forge gate --dry-run` rehearse, then full bounded `forge gate --timeout-secs 600`; verdict recorded; attributable failure blocks — else recorded as pre-existing with remediation.
- [ ] Archive with `openspec archive web-agent-identity-readonly --yes` (no `--skip-specs`); HANDOFF evidence + pointer handling; EXACTLY 2 commits (implementation+tests+package+spec, then HANDOFF only); no push.
