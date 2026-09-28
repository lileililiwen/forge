# Proposal: portal web UI to list and manage projects

## Why

Forge manages the fleet today through CLI text (`forge list`,
`forge fleet list`, `forge deploy status`) and ad-hoc JSON parsing.
An operator asking "which projects exist, which are online, and
which need a republish" must join three commands by hand — the same
join the `fleet-liveness-status` package types for machines. A
browser UI that lists projects with live health and offers the few
safe managements (inspect, republish, deploy-status drill-down)
makes the fleet operable without SSH fluency, while the CLI/MCP
remain the full-power surfaces.

## What Changes

- Serve a browser UI from Forge itself: `GET /ui` (fleet project
  list with per-project publish state + liveness verdict), `GET
  /ui/projects/{id}` (detail: manifest, maturity, doctor summary,
  journal, liveness), and `POST /ui/projects/{id}/publish`
  (confirm-gated republish that enqueues the same
  `RemoteComposeAdapter` lane as `forge publish`, returning the
  operation identity for tracking).
- Server-rendered HTML from the existing API service
  (`forge api serve`): no JavaScript framework, no new runtime
  dependency, no build step — the UI is plain HTML forms + links
  over the same Core validation, authorization (per-project OIDC
  sessions), confirm gates, and idempotency the API already
  enforces. The ROADMAP-deferred graphical framework choice
  (ASP.NET Core / Next.js) stays deferred; this package must not
  lock it.
- Read-mostly: list/detail/status pages are reads; the single
  mutation (republish) reuses the Core publish path with identical
  preconditions, evidence, and journaling.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `portal-web-ui` (Forge Rust) | Browser UI lists all projects with live health and manages republish through Core gates | Forge Rust, `src/api/` + `src/portal/`; no new service, no new repo | HTML over existing API auth/validation; `forge-fleet-liveness/0.1.0` consumed for the online column | `core-http-api`, `control-plane-portal`, `fleet-liveness-status`, `decoupled-remote-publish` | CLI contract + HTTP tests (list renders roster, detail renders evidence, republish without confirm refused, with confirm enqueues) + live browser capture |

Single outcome (operable project list in a browser), single owner,
single lifecycle (request/response over the API service). Split
signals considered: the liveness join is its own package
(`fleet-liveness-status`, consumed here, not reimplemented); a
graphical-framework portal is a deferred downstream choice,
explicitly not this package; CLI/MCP parity is owned by their
existing specs. Smallest independently verifiable unit: list +
detail + one gated action, all testable over HTTP without a
browser engine.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `forge api serve` | `src/api/`, `core-http-api` spec | Loopback listener, OIDC sessions, confirm-gated mutations, idempotency, bounded bodies | No HTML surface today | Forge | **Extend shared owner** (new `Accept: text/html` views + 3 UI routes on the same service/auth) |
| `forge portal dashboard/view` | `src/portal/`, `control-plane-portal` spec | Twelve-section read model, worst-status rollup, read journaling | CLI text only, no browser, no actions | Forge | **Extend shared owner** (portal view models feed the HTML pages) |
| Fleet liveness report | `fleet-liveness-status` (this cycle) | Typed online verdicts per project | Ships as CLI JSON; UI needs it per row | Forge | **Adopt** (UI consumes the report; no second join) |
| Next.js/ASP.NET portal | ROADMAP deferred choice | None in tree | Would add a service, language, and release boundary | Deferred | **Keep local** (server-rendered HTML now; framework migration stays a downstream option with the API contract as the seam) |

## BFS Impact Map

| Surface | Impact |
|---|---|
| API service | +3 UI routes on the same listener/auth/bounds; content negotiation (`Accept: text/html` vs JSON); no new port, no new auth model |
| Portal views | Existing `dashboard/view` models reused as HTML data; CLI output byte-identical |
| Publish lane | Republish calls the same Core entry as `forge publish` (same preconditions/evidence/journal); no new publish semantics |
| Liveness | One `fleet online` evaluation per list render (bounded, cached per request only — no retention) |
| MCP/CLI | Untouched |
| Failure surface | Unauthenticated/unauthorized/forbidden render the portal error page (never a redirect loop); API-down renders the boundary message per the portal spec; partial fleet results render per-row states, never a whole-page failure |
| Tests | HTTP contract tests (no browser engine): status codes, row counts vs roster, gated-action refusal/acceptance, XSS escaping of project-controlled strings |

## Capabilities

### New Capabilities

- `portal-web-ui`: browser project list with live health plus detail and confirm-gated republish, served from the existing API service as framework-free HTML.

### Modified Capabilities

(none — `core-http-api` and `control-plane-portal` keep their SHALLs; the UI projects their contracts into HTML)

## Non-goals

- A JavaScript SPA, a new service/language/framework, or any choice that pre-empts the deferred graphical portal decision.
- Editing project source, manifests, specs, or agent sessions from the UI (inspect + republish only).
- Replacing CLI/MCP; every UI action stays available there (portal spec boundary).
- Uptime history, alerting, multi-user management, or SSO provider work (existing identity only).
