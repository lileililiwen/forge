# Design: portal-web-ui

## Context

`core-http-api` serves JSON over a loopback listener with OIDC
sessions and confirm-gated mutations; `control-plane-portal`
defines the twelve-section read model plus accessible-operation
requirements (plan display, operation tracking, partial-result
states). This design projects those two contracts into
**`maud`-rendered** HTML: the operator gets list/manage in a
browser today, and a future framework portal can replace the
rendering layer against the unchanged API seam.

## Goals / Non-Goals

**Goals:**
- Open `http://127.0.0.1:8765/ui`, see every fleet project with
  its latest journal publish state, drill into one, republish
  it with an explicit confirm, track the operation id.
- One new dep tree (`maud` + `maud_macros` + `itoa`), registered
  in `deny.toml` with reasons; zero new ports/auth; zero build
  step beyond `cargo build`.
- Every string from project-controlled sources (names, notes,
  evidence) is HTML-escaped by maud's default escaping; every
  mutation re-passes Core gates.

**Non-Goals:** SPA/framework, source editing, CLI replacement,
per-row live liveness probing from the UI process (it is the
CLI/API's job; live liveness SSH-probes the Mac), history /
alerting, new identity work (see proposal).

## Decisions

### D1 — `maud` (compile-time HTML) inside `forge api serve`

**Chosen:** Rust, same crate/toolchain, with `maud 0.27` as the
renderer. New module `src/api/ui/` (one file per concern:
`render.rs` for the maud templates + layouts, `routes.rs` for
the three new HTTP routes, `auth.rs` for the same-session
authorization binding, `error.rs` for the typed HTML error
pages). The routes hook into the existing `forge api serve`
listener at `/ui`, `/ui/projects/{id}`,
`/ui/projects/{id}/publish`. Content negotiation:
`Accept: text/html` serves the maud-rendered page;
`Accept: application/json` returns the existing JSON envelope
unchanged. Styling: inline `<style>` (~2 KiB, system fonts, no
external assets so the page works offline and air-gapped). No
JavaScript at all — forms and links only (matches the portal
spec's keyboard-accessible requirement).

**Alternatives considered:**
- **Hand-rolled `Element` tree** (a typed enum + helper
  functions). Tried first; rejected because every new page is
  a new top-down write with no layout/component reuse, no
  template inheritance, no built-in escaping discipline —
  exactly the friction this proposal exists to remove.
- **Raw `format!` strings.** Tempting but loses compile-time
  validation, easy to forget `escape_html`, no separation of
  structure from data. Rejected.
- **`askama`** (Jinja-style templates in `.html` files).
  Rejected for this change: requires separate template files
  (sync friction with the Rust source), pulls a heavier
  dep tree (`askama_derive`, `askama_escape`, `itoa`), and
  the JSX-shaped `maud` macro is closer to the Next.js-style
  component model that motivated this proposal.
- **Next.js app** (real JavaScript dependency). Rejected as a
  pre-emption of the deferred graphical framework decision
  and because it would add a Node.js service + language +
  release boundary. The `maud` choice is the in-Rust
  equivalent without crossing that line.

### D2 — Route and behavior model

| Route | Behavior |
|---|---|
| `GET /ui` | 200 HTML page: one row per `compose_ready` roster project. Columns: project id (link to detail), profile, latest `publish` journal row state (done/failed/...) | 200, latest timestamp; `subdomain` link from the inventory snapshot. Skipped entries section reusing the fleet wording. The page reads only the registry (no SSH, no target access). |
| `GET /ui/projects/{id}` | 200 HTML detail: identity/manifest, maturity, doctor summary, latest publish / deploy journal rows. Unknown id → 404 portal error page (same shape as the API 404 but HTML). |
| `POST /ui/projects/{id}/publish` | Without `confirm=yes` body → 200 plan-preview page (the dry-run `forge publish all` report rendered as HTML) with a confirm form. With confirm → 303 redirect to the detail page with a flash message and one new `publish` journal row. Idempotency-Key honored exactly like the API (same `reserve_idempotent_operation` call). |

**v0 scope reduction:** the original proposal included a
"per-row liveness verdict" column on the list page. That
requires SSH-probing the Mac for every list render, which
neither the loopback listener nor the in-process UI can do
without contacting the operator's target. Live liveness
stays the `fleet-liveness-status` package's surface and
remains reachable through `forge fleet online` / the API
JSON. The list page shows the latest journal `publish`
state per project instead, which is the registry's source
of truth and never requires target access.

**Alternatives considered:**
- **GET-triggered publish** (a `GET /ui/projects/{id}/republish`
  link that does the work). Rejected: violates safe-method
  semantics and the confirm-gate requirement.
- **JSON-only API at `/ui/...` + a separate SPA** (React,
  Solid, Leptos hydration). Rejected: SPA needs JS in the
  browser, a build step, and pre-empts the deferred
  graphical framework decision. maud-rendered HTML is the
  in-Rust equivalent.

### D3 — Auth, gates, bounds

**Chosen:** the API's existing bearer-token authorization
applies unchanged. The UI routes reuse
`forge::api::authorize()` exactly as the JSON routes do:
unauthenticated → portal login page (HTML, never a JSON
leak); unauthorized project → 403 HTML error page. Bounded
bodies (API `MAX_BODY_BYTES = 1 MiB`) apply unchanged.

**CSRF defense:** the API uses bearer tokens, not cookies, so
classic cookie-CSRF is not in scope. The browser POSTs the
bearer token as a hidden form field, and the dispatcher
re-checks it on the POST. Additionally, the dispatcher
verifies that the request `Origin` (when present) matches the
loopback bind address (e.g. `http://127.0.0.1:8765`); a
cross-origin form post is refused. This gives equivalent
protection to a same-origin session cookie + CSRF token, and
matches the API's existing authorization tests.

**HTML escaping:** every project-controlled string flows
through maud's default escape (the `{value}` interpolation
escapes; `PreEscaped` is the explicit opt-out and is only
used for the trusted inline `<style>` block). Unit tests
cover the escaping matrix (quotes, tags, unicode, control
characters).

**Secret redaction:** every journal row string passes through
`policy::redact_credentials` before render (same as CLI
evidence). Liveness probing is out of scope for this change
(D2).

### D4 — Contract and compatibility

**Chosen:** no new versioned contract. The HTML is a
projection of the existing journal / registry / portal view
models, which already have versioned contracts (`forge-fleet-liveness/0.1.0`,
the journal columns, the portal sections). `Accept:
application/json` on UI paths returns the underlying JSON
unchanged (content negotiation, no fork). CLI / MCP outputs
stay byte-identical (maud-rendered pages are additive).

### D5 — Failure and boundary policy

| Case | Behavior |
|---|---|
| API service down | Browser connection refused (unchanged); CLI remains (portal spec boundary) |
| Unknown project on detail | 404 HTML error page (never a 500, never a JSON leak to a browser) |
| Republish preconditions fail | 200 plan-preview page with the typed refusal message; nothing enqueued |
| Concurrent republish | Idempotency-Key dedupes; second submit returns the same operation id |
| Cross-origin form POST | 403 HTML error page; bearer token never leaks to a different origin |

### D6 — Verification oracle

- Unit (`src/api/ui/render.rs`): maud template rendering
  smoke tests for list / detail / plan-preview / operation
  accepted / 404 / 403; escape matrix (quotes, tags,
  unicode, control characters) on adversarial project
  names / notes / evidence; layout composition tests.
- Unit (`src/api/ui/auth.rs`): bearer-token re-check,
  cross-origin check, idempotency-key dedupe.
- HTTP contract (`tests/portal_ui_contract.rs`, no browser
  engine): list row count equals fixture roster; detail
  404s unknown ids; publish without confirm returns the
  plan page and enqueues nothing (journal diff zero); with
  confirm returns 303 redirect + the journal row appears;
  project-controlled strings arrive escaped; cross-origin
  POST is refused.
- Live: `curl` capture of the three pages against
  `forge api serve --bind 127.0.0.1 --port 18765` with a
  fixture registry (operator-runnable, no Mac required).
  This replaces the original "browser screenshot against the
  Mac fleet" task, which required target access the
  in-process UI does not have.

### D7 — Risks / trade-offs

- **No live liveness in the list** → Mitigation: the CLI
  and API still expose `forge fleet online` and the
  `forge-fleet-liveness/0.1.0` document; the detail page
  links to the same. Future package can extend the list to
  consume the liveness report from a sibling cache if the
  target access story changes.
- **HTML in the API service blurs layers** → Mitigation:
  maud templates are pure render functions over existing
  view models; JSON untouched; a future framework migration
  replaces `src/api/ui/`, not Core.
- **Three new crates in the dep closure** → Mitigation:
  all three registered in `deny.toml` with explicit
  reasons; transitive `itoa` is the same crate already used
  by askama-style renderers across the ecosystem; no
  version surprise; build graph stays reproducible.
- **Hidden bearer token in HTML forms** → Mitigation: the
  token is the operator's session id (already bearer-shared
  over the JSON API); same-origin check on `Origin`
  prevents cross-origin exfiltration; the operator can
  rotate the session id through the existing identity
  surface.

## Migration Plan

Additive: new module, three routes, tests, three new
workspace dependencies + `deny.toml` entries. No persistence
change, no config change (UI lives at the API bind address),
no sibling impact. Rollback: remove `src/api/ui/`, the three
route entries, and the new deps; the API surface reverts to
JSON-only.

## Open Questions

None — the maud choice, the v0 scope reduction, and the
auth/CSRF model are all explicit in this document and the
proposal.