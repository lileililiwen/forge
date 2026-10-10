# Proposal: Web shipping provider reads

## Why

Five shipping/provider/fleet reads — `publish provider list|inspect`,
`provider matrix|inspect`, `plugins list` — sit at `not_yet_web` in the
command catalog while needing no provider probing, no network, no
adapter invocation and no secret: they are local-config/registry
projections over `.forge/providers.yaml` (`publish::providers::
load_config`), the static evidence-provider table (`provider::matrix`
with `live=false`, `provider::inspect`), and the plugin registry
(`plugins::list` over the same config plus its sibling `plugins:`
block). (`fleet inspect` is already web via
`GET /v1/admin/fleet/{entry}` and is reused as-is.) An operator auditing
"which publish providers and plugins are configured, and what the
provider matrix says" must drop to the terminal. This is web UI/UX audit
gap 8.

All five reads are safe as session-gated read-only GETs reusing the
existing Core stores. Every mutating or probing verb stays where it is
by design: `publish sync|db|fleet|prepare|deploy|all`, `docs translate`,
`provider run`, `fleet online`, `project.github *`, `push`/`mirror`
(git), `readiness *` (native toolchain), `deploy observe`
(capability-gated), transports, and agent/identity lifecycle writes gain
no route here. Provider `enable|disable` stays `not_yet_web`.

## What Changes

- Five `not_yet_web` rows become `web` with typed read-only
  session-gated admin GETs (no adapter, no probe, no network, no write,
  no journal row, no browser-supplied path):
  - Project-bound (directory resolved server-side via the existing
    `deploy_id_gate`, absolute paths scrubbed): `GET
    /v1/admin/projects/{id}/publish/providers`
    (`publish::providers::load_config` over the server-resolved
    `.forge/providers.yaml`, missing file → honest
    `unavailable-with-reason`); `GET
    .../publish/providers/{provider}` (one entry, unknown → typed
    `404`); `GET /v1/admin/projects/{id}/plugins` (`providers::
    load_config` + `plugins::load_descriptors` + `plugins::list`,
    missing file → honest empty-registry answer mirroring the CLI).
  - Pure global (no project): `GET /v1/admin/providers/matrix`
    (`provider::matrix(false)` — every row `not-run`, never a support
    claim, no journal row unlike the CLI); `GET
    /v1/admin/providers/{provider}` (`provider::inspect` static
    descriptor, unknown → typed error, hostile segment → static `400`).
- One `route_shipping_reads` matcher kept beside the handlers in the new
  `src/api/admin/shipping_reads.rs` — never in the `router.rs` table
  (999/1000 lines, no new variant, no new table/permission/authorize
  arm). The three new sections ride the shared `Route::AdminCreation`
  validated-key triple (`shipping-providers`, `evidence-providers`,
  `shipping-plugins` — disjoint from every creation/assurance key), and
  `assurance::dispatch_creation` gains one three-way branch
  (constant-line-count `deploy.rs` untouched). One `route_beside` fold
  line in `src/api/admin/mod.rs`.
- `IMPLEMENTED_WEB_ROUTES` +5 with route refs; five `NotYetWeb` rows →
  `web_at(Read, route, caps)` (count stays 234, `problems()` empty).
- SPA: one compact read-only "Provider & plugin reads" card inside the
  `#delivery` view (after the history card, before confirmed actions)
  with project input, five explicit loaders, `role="status"` results,
  honest unavailable-with-reason states, and a link-out to the existing
  fleet-inspect control for `fleet inspect`; no new dependency, no
  `innerHTML`.
- Contract tests: new `tests/web_shipping_provider_reads_contract.rs`
  (≈10 tests: auth, gates, five lists/inspects incl. typed refusals,
  matrix not-run parity, plugins empty-registry, scrub, catalog pins).

## BFS Impact Map

- Requirements/scenarios: 5 reads across 3 sections; each maps to one
  route constant, one `Route` key triple, one handler, one catalog-row
  conversion, one frontend loader, one contract-test assertion.
- Modules: new `src/api/admin/shipping_reads.rs`; `routes.rs` (+5
  constants); `router.rs` (untouched — constant line count);
  `assurance.rs` (+1 dispatch branch); `admin/mod.rs` (+1 fold line);
  `deploy.rs` (untouched); `handlers_core.rs` (untouched);
  `command_catalog` (rows_shipping conversions, `IMPLEMENTED_WEB_ROUTES`
  +5); `frontend/index.html|app.js|styles.css` (one card, additive only).
- Contracts/persistence: no Core logic change; no registry write; no
  journal row; registry bytes identical before/after reads (asserted
  in-contract).
- Callers/integrations: CLI/MCP/Core bytes unchanged; no provider,
  adapter, shell, network, or file/stdin path crosses the boundary.
- Tests/compat: new contract suite + lib + CLI suites
  (publish/provider/plugins) + existing web contracts; live
  throwaway-registry oracle with zero JS errors; gate dry-run + bounded
  full gate before archive.
- Concerns: `.ai-rules/concerns/quality.md` (read-only parity),
  `.ai-rules/concerns/security.md` (session gate, id gates, scrub, no
  echo, exact origin); architecture (modular monolith, thin transports).

## Capabilities

- Local filesystem reads through Core stores only; registry reads.
- Loopback bind for the existing API/web listeners (no new listener).
- No external provider capability (by omission — the point).

## Non-goals

- Any provider invocation, enable/disable/mutate, live probe, or secret
  output: `publish sync|db|fleet|prepare|deploy|all`, `docs translate`,
  `provider run`, `fleet online`, `project.github *`, `push`/`mirror`
  (git), `readiness *` (native toolchain), `deploy observe`
  (capability-gated), transports, and agent/identity lifecycle writes
  stay CLI-or-existing-web and gain no route here.
- `publish provider enable|disable` stays `not_yet_web`.
- No browser-supplied config path, binary, argv, host, SSH target, or
  credential on any path; the provider id, project and revision
  resolution of the existing publish apply route is untouched.
