# Proposal: portal web UI to list and manage projects

## Why

Forge manages the fleet today through CLI text (`forge list`,
`forge fleet list`, `forge deploy status`) and ad-hoc JSON parsing.
An operator asking "which projects exist, which are online, and
which need a republish" must join three commands by hand — the same
join the `fleet-liveness-status` package types for machines. A
browser UI that lists projects with their latest publish state and
offers the few safe managements (inspect, republish, deploy-status
drill-down) makes the fleet operable without SSH fluency, while
the CLI/MCP remain the full-power surfaces.

## What Changes

- Serve a browser UI from Forge itself: `GET /ui` (fleet project
  list with per-project publish state from the journal), `GET
  `/ui/projects/{id}` (detail: manifest, maturity, doctor summary,
  journal rows), and `POST /ui/projects/{id}/publish`
  (confirm-gated republish that enqueues the same
  `RemoteComposeAdapter` lane as `forge publish`, returning the
  operation identity for tracking).
- Server-rendered HTML from the existing API service
  (`forge api serve`). Rendered through **`maud`** — the
  JSX-shaped compile-time HTML crate, the standard idiomatic
  choice for server-rendered HTML in Rust. The renderer is a
  single new transitive dependency tree (`maud` + `maud_macros`
  + `itoa`); the templates live inline in the Rust source
  (no separate `.html` files, no build step beyond `cargo
  build`). The `cargo deny` closure changes by exactly these
  three crates, registered in `deny.toml` with reasons; this
  is the only way to ship a real web framework without
  inventing one. The ROADMAP-deferred graphical framework
  choice (ASP.NET Core / Next.js) stays deferred; maud is
  the closest Rust equivalent and does not lock it.
- Read-mostly: list/detail/status pages are reads; the single
  mutation (republish) reuses the Core publish path with
  identical preconditions, evidence, and journaling.
- **v0 scope reduction** (documented in design D2 below):
  per-row live liveness probing (SSH into the Mac per render)
  is **out of scope** for this package. The list page shows
  the most recent journal `publish` row state per project,
  which is the registry's existing source of truth and is
  available without contacting the target. Live liveness is
  consumed through the CLI/API surfaces instead (no behaviour
  regression for operators who already use `forge fleet online`).

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `portal-web-ui` (Forge Rust) | Browser UI lists all projects with journal publish state and manages republish through Core gates | Forge Rust, `src/api/ui/` + `src/api/mod.rs`; no new service, no new repo | HTML over existing API auth/validation; maud-rendered; one new dep tree | `core-http-api`, `control-plane-portal`, `decoupled-remote-publish` | CLI contract + HTTP tests (list renders roster, detail renders evidence, republish without confirm refused, with confirm enqueues) + escape matrix + screenshot capture from `curl` against `forge api serve` |

Single outcome (operable project list in a browser), single
owner, single lifecycle (request/response over the API service).
Split signals considered: per-row live liveness probing is
intentionally split out — it is the `fleet-liveness-status`
package's surface and would require SSH/target access on every
list render, which neither the registry nor the operator's
loopback listener can guarantee; the CLI/API surfaces keep the
typed online verdicts. A graphical-framework portal is a
deferred downstream choice, explicitly not this package; maud
is what is closest to JSX in Rust without leaving the language
and does not lock that choice. CLI/MCP parity is owned by their
existing specs. Smallest independently verifiable unit: list +
detail + one gated action, all testable over HTTP without a
browser engine.

## Dependency boundary and `cargo deny`

This proposal lifts the previous "no new dependency" rule for
exactly one purpose: a real server-rendered HTML framework. The
chosen crate is `maud` (MIT OR Apache-2.0, the most idiomatic
JSX-shaped HTML in Rust, no template files, compile-time only).
The transitive closure is `maud` + `maud_macros` + `itoa`. All
three are added to `[workspace.dependencies]` and registered
in `deny.toml` with explicit reasons:

| Crate | Licence | Reason |
|---|---|---|
| `maud` | MIT OR Apache-2.0 | JSX-shaped compile-time HTML for the in-process portal UI; replaces a hand-rolled element tree. |
| `maud_macros` | MIT OR Apache-2.0 | Proc-macro companion to `maud`; same publisher (lambda-fairy). |
| `itoa` | MIT OR Apache-2.0 | Required by `maud` for integer formatting; transitive only. |

No other change to the dependency closure. Every prior
"closure unchanged" verification in HANDOFF stays true for the
crate set other than these three entries.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `forge api serve` | `src/api/`, `core-http-api` spec | Loopback listener, OIDC sessions, confirm-gated mutations, idempotency, bounded bodies | No HTML surface today | Forge | **Extend shared owner** (3 UI routes on the same service/auth) |
| `forge portal dashboard/view` | `src/portal/`, `control-plane-portal` spec | Twelve-section read model, worst-status rollup, read journaling | CLI text only, no browser, no actions | Forge | **Extend shared owner** (portal view models feed the HTML pages) |
| Fleet liveness report | `fleet-liveness-status` | Typed online verdicts per project | Probe is SSH-bound to Mac; in-process UI cannot probe per render | Forge | **Adopt as data only** (UI documents the latest journal state; liveness join stays a CLI/API call) |
| `maud` | `crates.io/crates/maud` | Compile-time HTML, JSX-shaped, no template files, no runtime reflection | Adds three crates to the dep closure | lambda-fairy | **Adopt as the renderer** (one new dep tree, justified above) |
| Next.js/ASP.NET portal | ROADMAP deferred choice | None in tree | Would add a service, language, and release boundary | Deferred | **Keep deferred** (maud is the in-Rust equivalent; framework migration stays a downstream option with the API contract as the seam) |
| Hand-rolled element tree | Earlier typed `Element` helper | No new dep | Hard to extend per page, no layout reuse, no template inheritance — exactly the friction this proposal exists to remove | Forge | **Reject** (replaced by maud) |

## BFS Impact Map

| Surface | Impact |
|---|---|
| `Cargo.toml` / `[workspace.dependencies]` | +`maud = "0.27"` (and its two transitive companions via feature graph; explicit entries for the proc-macro and `itoa`) |
| `deny.toml` | +three allow-list entries with reasons (table above) |
| API service | +3 UI routes on the same listener/auth/bounds; content negotiation (`Accept: text/html` vs JSON); no new port, no new auth model |
| Portal views | Existing `dashboard/view` models reused as HTML data; CLI output byte-identical |
| Publish lane | Republish calls the same Core entry as `forge publish` (same preconditions/evidence/journal); no new publish semantics |
| Liveness | Out of scope for this change (see "v0 scope reduction"); UI shows the latest journal `publish` row state |
| MCP/CLI | Untouched |
| Failure surface | Unauthenticated/unauthorized render the portal error page (never a redirect loop); API-down renders the boundary message per the portal spec |
| Tests | HTTP contract tests (no browser engine): status codes, row counts vs roster, gated-action refusal/acceptance, escape matrix on project-controlled strings |

## Capabilities

### New Capabilities

- `portal-web-ui`: browser project list with latest journal publish state, project detail, and confirm-gated republish, served from the existing API service as maud-rendered HTML.

### Modified Capabilities

(none — `core-http-api` and `control-plane-portal` keep their SHALLs; the UI projects their contracts into HTML)

## Non-goals

- A JavaScript SPA, a new service/language/framework, or any choice that pre-empts the deferred graphical portal decision.
- Per-row live liveness probing from the UI process (SSH into the Mac per render is `fleet-liveness-status`'s job; CLI/API still expose it).
- Editing project source, manifests, specs, or agent sessions from the UI (inspect + republish only).
- Replacing CLI/MCP; every UI action stays available there (portal spec boundary).
- Uptime history, alerting, multi-user management, or SSO provider work (existing identity only).