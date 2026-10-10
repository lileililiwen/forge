# Proposal: Web assurance browser

## Why

The assurance reads — `spec list|inspect|route`, `remediate scan|diff`,
`describe list|show`, `classify list|show`, `contract list|inspect|emit`,
`governance list|status|inspect`, `analytics metrics`, `studio preview`
(status read) — are CLI-only: seventeen command-catalog rows sit at
`not_yet_web`. The dashboard's command reference shows them as "Not in
web yet" and the projects view has no assurance section. An operator
auditing "which specs/remediations/proposals/contracts/governance/
metrics/preview state exist for this project" must drop to the terminal.
This is web UI/UX audit gap 7.

All seventeen reads are deterministic Core store projections — spec
manifests under `.forge/specs/`, `remediation::scan|diff` over the
server-resolved target, semantic `decide::list|read` under
`.forge/semantic/`, vendored contract manifests + Core emit helpers,
`governance::list_providers|evaluate_project` (read-only, no persist),
registry-only `analytics::aggregate_project_metrics` (no provider probe,
no summary write, no journal), and `studio::load_session` +
`envelope_from_session` (no spawn, no port bind). They are safe as
session-gated read-only GETs. Every write and every provider/shell/
browser-path boundary stays where it is by design.

## What Changes

- Seventeen `not_yet_web` rows become `web` with typed read-only
  session-gated admin GETs (no write, no provider probe, no adapter run,
  no shell, no journal row, no browser-supplied path):
  - Pure contract catalog (no project): `GET /v1/admin/contracts`
    (`contract::CONTRACTS` + `load_manifest`) and
    `GET /v1/admin/contracts/{family}` (`family_schema_path` + vendored
    schema `required` projection; unknown family → typed `400`).
  - Project-bound reads (directory resolved server-side via the existing
    `deploy_id_gate`, absolute paths scrubbed):
    `GET /v1/admin/projects/{id}/specs` (`spec::list_specs`),
    `GET .../specs/{spec}` (`spec::read_spec`, unknown → typed `404`,
    prefix match mirrors the CLI),
    `GET .../spec/route?finding=` (`spec::route_finding` over a
    CLI-parity finding source: `driftwatch-` policy prefix, else doctor
    fail input; no `apply`),
    `GET .../remediate/scan` (`remediation::scan`),
    `GET .../remediate/diff?finding=&pack=` (rebuild via
    `remediation::build_plan` then `remediation::diff`; no `--plan`
    file, no browser path),
    `GET .../describe/proposals` + `GET .../describe/proposals/{proposal}`
    (`semantic::decide::list|read`, description kind),
    `GET .../classify/proposals` + `GET .../classify/proposals/{proposal}`
    (`semantic::decide::list|read`, domain kind),
    `GET .../contracts/emit?family=` (family allowlist
    `supported_families()`; gate-result/release-evidence project the
    server-resolved dir, readiness empty → typed unavailable, unknown
    family → typed `400`),
    `GET .../governance` (`governance::list_providers`),
    `GET .../governance/status` + `GET .../governance/inspect`
    (`governance::evaluate_project`, never `check_project`; external
    enabled provider → honest `unavailable-with-reason`, never an
    adapter run),
    `GET .../analytics/metrics?window_days=` (registry-only
    `aggregate_project_metrics` with empty externals +
    `DoctorSummary::default()`; window clamped to
    `[MIN_WINDOW_DAYS, MAX_WINDOW_DAYS]`; no `inspect_external_planes`,
    no summary write, no journal),
    `GET .../studio/preview` (`studio::load_session` +
    `envelope_from_session`; none → `state: none` envelope, never
    start/stop/spawn).
- One shared `Route::AdminCreation { registry, item, action }` validated-key
  triple (portfolio `{kind}`/`{action}` + creation-catalog precedent) plus a
  `route_assurance` matcher kept beside the handlers in the new
  `src/api/admin/assurance.rs` — never in the `router.rs` table (constant
  999/1000 lines, no new variant, no new table/permission/authorize arm).
  One `dispatch_creation` branch in `src/api/admin/deploy.rs` (constant
  line count) sends the nine disjoint assurance sections to
  `assurance::dispatch`.
- `IMPLEMENTED_WEB_ROUTES` +17 with route refs; seventeen `NotYetWeb`
  rows → `web_at(Read, route, caps)` (count stays 234, `problems()`
  empty).
- SPA: one read-only "Assurance" card (`<section id="assurance-browser">`
  inside `#view-projects`, after `#creation-catalog`) with a per-registry
  picker (spec / remediate / describe / classify / contract / governance /
  analytics / studio), list/inspect/route-or-scan-or-diff-or-emit-or-
  metrics-or-preview rendering, explicit reads only, honest
  unavailable-with-reason states; no new dependency, no `innerHTML`.
- Contract tests: new `tests/web_assurance_browser_contract.rs`
  (≈12 tests: auth, gates, seventeen lists/inspects incl. typed refusals,
  scan/diff incl. rebuild parity, governance external-unavailable,
  analytics window clamp + no-write, studio none-envelope, scrub, catalog
  pins for all 17 rows + CLI-only leftovers).

## BFS Impact Map

- Requirements/scenarios: 17 reads across 8 registries; each maps to one
  route constant, one `Route` key triple, one handler, one catalog-row
  conversion, one frontend loader, one contract-test assertion.
- Modules: new `src/api/admin/assurance.rs`; `routes.rs` (+17 constants);
  `router.rs` (+1 chain line, cap 1000); `deploy.rs` (+1 dispatch arm);
  `handlers_core.rs` (+1 authorize arm); `command_catalog` (rows_assurance
  + rows_platform conversions, `IMPLEMENTED_WEB_ROUTES` +17);
  `frontend/index.html|app.js|styles.css` (one section, additive only).
- Contracts/persistence: no Core logic change; no registry write; no
  journal row; no summary file; no observation persist; registry bytes
  identical before/after reads (asserted in-contract).
- Callers/integrations: CLI/MCP/Core bytes unchanged; no provider, adapter,
  shell, port bind, or file/stdin path crosses the boundary.
- Tests/compat: new contract suite + lib + CLI suites
  (spec/remediate/semantic/contract/governance/analytics/studio) +
  existing web contracts; live throwaway-registry oracle with zero JS
  errors; gate dry-run + bounded full gate before archive.
- Concerns: `.ai-rules/concerns/quality.md` (read-only parity),
  `.ai-rules/concerns/security.md` (session gate, id gates, scrub, no
  echo, exact origin); architecture (modular monolith, thin transports).

## Capabilities

- Local filesystem reads through Core stores only; registry reads.
- Loopback bind for the existing API/web listeners (no new listener).
- No external provider capability (by omission — the point).

## Non-goals

- Any new write/apply/approve/use verb: `spec generate|apply`,
  `remediate plan|apply`, `classify apply|approve|reject` are already web
  and untouched; `describe suggest|approve|reject`, `classify
  suggest|derive`, `contract validate` (file/stdin), `governance use`,
  `analytics inspect` (provider), `studio spec|refine` flows stay
  CLI-or-existing-web and gain no new write route here.
- Provider-backed `analytics inspect`, file/stdin `contract validate`,
  external governance adapter runs, preview start/stop/spawn, remediation
  `--plan` file uploads, browser-supplied paths, journal writes,
  observation persists, summary writes.
