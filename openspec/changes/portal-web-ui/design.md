# Design: portal-web-ui

## Context

`core-http-api` serves JSON over a loopback listener with OIDC
sessions and confirm-gated mutations; `control-plane-portal`
defines the twelve-section read model plus accessible-operation
requirements (plan display, operation tracking, partial-result
states). This design projects those two contracts into
framework-free HTML: the operator gets list/manage in a browser
today, and a future framework portal can replace the rendering
layer against the unchanged API seam.

## Goals / Non-Goals

**Goals:**
- Open `http://127.0.0.1:8765/ui`, see every fleet project with
  publish state + online verdict, drill into one, republish it
  with an explicit confirm, track the operation id.
- Zero new dependencies, zero build step, zero new ports/auth.
- Every string from project-controlled sources (names, notes,
  evidence) is HTML-escaped; every mutation re-passes Core gates.

**Non-Goals:** SPA/framework, source editing, CLI replacement,
history/alerting, new identity work (see proposal).

## Decisions

### D1 — Server-rendered HTML inside `forge api serve`

**Chosen:** Rust, same crate/toolchain. New `src/api/ui.rs`
(render functions pure over portal/liveness view models) + three
routes in `src/api/mod.rs` (`GET /ui`, `GET /ui/projects/{id}`,
`POST /ui/projects/{id}/publish`) served only when the request
`Accept`s `text/html` (JSON behavior unchanged). Styling: inline
`<style>` (~2 KiB, system fonts, no external assets so the page
works offline and air-gapped). No JavaScript except zero — forms
and links only (matches the portal spec's keyboard-accessible
requirement trivially).

**Alternatives:** Next.js app (rejected: new language/service/
release boundary; pre-empts the deferred framework decision);
embedded SPA bundle (rejected: build step + JS payload for three
pages); separate `forge portal serve` binary (rejected: splits
auth/config from the API service for no benefit).

### D2 — Route and behavior model

| Route | Behavior |
|---|---|
| `GET /ui` | 200 HTML table: one row per `compose_ready` roster project (plus skipped-entry section reusing fleet wording): columns project, profile, publish state (latest journal), online verdict (one bounded `fleet online` evaluation per render, 12s default cap each, failures render per-row `unverified`), subdomain link. Reads journaled as portal reads per the portal spec. |
| `GET /ui/projects/{id}` | 200 HTML detail: identity/manifest, maturity, latest doctor summary, latest deploy/publish journal rows, liveness row, operation history links. Unknown id → 404 portal error page (same shape as API 404, HTML). |
| `POST /ui/projects/{id}/publish` | Without `confirm=yes` body → 200 plan-preview page (the exact stages Forge would run, dry-run rendering) with a confirm form. With confirm → 202 enqueues the Core publish lane (identical to `forge publish all <id>` incl. revision capture, queue row, journaling) and renders the operation id + tracking link. Idempotency-Key honored exactly like the API. |

**Alternatives:** GET-triggered publish (rejected: violates safe-method semantics and the confirm-gate requirement).

### D3 — Auth, gates, bounds

**Chosen:** The API's existing session/authorization applies
unchanged (unauthenticated → portal login page, not a JSON leak;
unauthorized project → 403 portal error page). Bounded bodies
(API `max_body_bytes`), HTML-escaped outputs (escape helper with
unit tests over adversarial names/notes), no secret rendering
(redaction before render, same as CLI evidence). Liveness probing
from the UI process inherits the operator's target access (SSH +
tunnel reachability are the operator's environment, unchanged).

### D4 — Contract and compatibility

**Chosen:** No new versioned contract: HTML is a projection of
`forge-fleet-liveness/0.1.0`, the queue/journal rows, and portal
view models (all versioned where they live). `Accept:
application/json` on UI paths returns the underlying JSON
unchanged (content negotiation, no fork). CLI/MCP outputs
byte-identical (render functions are additive).

### D5 — Failure and boundary policy

| Case | Behavior |
|---|---|
| API service down | Browser connection refused (unchanged); CLI remains (portal spec boundary) |
| Liveness probe fails for a row | Row shows `unverified` + bounded detail; page still 200 (partial-result state per portal spec) |
| Republish preconditions fail | 200 plan page with typed refusal (same reason string as CLI), nothing enqueued |
| Concurrent republish | Idempotency-Key dedupes; second submit returns the running operation id |
| Unknown project | 404 HTML (never a 500, never a JSON leak to a browser) |

### D6 — Verification oracle

- Unit (`src/api/ui.rs`): HTML escaping matrix (quotes, tags,
  unicode), plan/confirm page rendering from fixtures, error-page
  shapes.
- HTTP contract (`tests/portal_ui_contract.rs`, no browser
  engine): list row count equals fixture roster; detail 404s
  unknown ids; publish without confirm returns the plan page and
  enqueues nothing (journal unchanged); publish with confirm
  returns 202 + operation id and the journal gains the row;
  project-controlled strings arrive escaped.
- Live: screenshots/capture of the three pages against the real
  API + Mac fleet (evidence, operator-run).

### D7 — Risks / trade-offs

- **Per-render liveness cost** → Mitigation: one bounded
  evaluation per list render, no retention; slow rows degrade to
  `unverified`, never block the page (documented).
- **HTML in the API service blurs layers** → Mitigation: pure
  render functions over existing view models; JSON untouched;
  framework migration later replaces `ui.rs`, not Core.
- **CSRF on the POST** → Mitigation: same-session form token
  (Origin + token check, mirroring the API's existing
  authorization tests); without it the POST is refused like an
  unauthorized call.

## Migration Plan

Additive: new module, three routes, tests. No persistence change,
no config change (UI lives at the API bind address), no sibling
impact. Rollback: stop serving `/ui*` (single route-table revert).

## Open Questions

None — framework choice stays explicitly deferred; all v1
decisions above are framework-free by construction.
