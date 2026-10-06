# portal-web-ui Specification

## Purpose

Serve a browser UI from `forge api serve` over the same
loopback listener and authorization the JSON API already
enforces. Three routes (`GET /ui`, `GET /ui/projects/{id}`,
`POST /ui/projects/{id}/publish`) deliver maud-rendered
HTML pages that project the registry's journal state and
the fleet read models, and the one mutation (republish) re-
passes Core gates with the same preconditions and
journaling as the CLI/MCP. The renderer is `maud 0.27`
(JSX-shaped compile-time HTML), so the pages are pure
functions of typed data with default escaping on every
project-controlled string. The package adds exactly three
crates to the dependency closure (`maud`, `maud_macros`,
`itoa`), all MIT/Apache-2.0, registered in `deny.toml` with
their rationale; no other transitive crate is added.

The package is read-mostly: list/detail are reads, the
single mutation (republish) reuses the Core publish path
with identical preconditions, evidence and journaling.
Per-row live liveness probing is intentionally out of scope
here — the list page consumes the registry's latest journal
`publish` row state, no SSH per render — and stays
reachable through `forge fleet online` and the API JSON.

The implementation does not invent a cookie session or a
CSRF token; the existing bearer-token auth applies
unchanged, the POST additionally re-checks the form's
hidden token and refuses cross-origin POSTs whose `Origin`
header (when present) does not match the loopback bind
address.
## Requirements
### Requirement: Browser project list with latest journal publish state

Forge SHALL serve `GET /ui` from the existing API service as
**`maud`-rendered** HTML listing every roster project with its
latest journal `publish` row state (done / failed / pending /
...) and the inventory subdomain when present, reusing API
authentication, portal read models, and the registry's journal.
Project-controlled strings SHALL be HTML-escaped by maud's
default escaping so an adversarial name or note cannot inject
markup.

#### Scenario: List renders the roster

- **WHEN** an authorized operator opens `/ui` with `Accept: text/html`
- **THEN** one row per `compose_ready` roster project shows project id (linking to the detail page), profile, latest publish state, latest publish timestamp, and subdomain link, plus the skipped-entry section, with project-controlled strings escaped

#### Scenario: JSON unchanged

- **WHEN** the same paths are requested with `Accept: application/json`
- **THEN** the pre-change JSON envelopes are byte-identical

#### Scenario: List does not contact the target

- **WHEN** the list page is rendered
- **THEN** no SSH / target command runs for any project on the list; the latest journal row is read from the registry only

### Requirement: Project detail with evidence

Forge SHALL serve `GET /ui/projects/{id}` rendering identity,
manifest, maturity, doctor summary, and journal rows for one
project, and SHALL render the portal 404 page for unknown ids.

#### Scenario: Unknown project

- **WHEN** a browser requests `/ui/projects/no-such-app`
- **THEN** the service answers 404 HTML without leaking JSON internals

#### Scenario: Detail renders evidence

- **WHEN** an authorized operator opens `/ui/projects/alethefy` with `Accept: text/html`
- **THEN** the page shows the project id, profile, maturity, doctor summary, latest `publish` and `deploy` journal rows, with project-controlled strings escaped

### Requirement: Confirm-gated republish from the browser

Forge SHALL serve `POST /ui/projects/{id}/publish` such that
without confirmation it renders the dry-run plan page and
enqueues nothing, and with confirmation it enqueues the Core
publish lane (same preconditions and journaling as `forge
publish`) and renders the operation identity for tracking.
Forge SHALL refuse the POST when the bearer token does not
match the form's hidden token or when the request `Origin`
header (when present) does not match the loopback bind address.

#### Scenario: Republish without confirm enqueues nothing

- **WHEN** the confirm form is submitted without `confirm=yes`
- **THEN** the journal gains no row and the response shows the dry-run plan with the typed refusal if preconditions fail

#### Scenario: Confirmed republish tracks

- **WHEN** the confirm form is submitted with `confirm=yes`
- **THEN** the response is 303 redirect to the detail page and the journal gains the matching `publish` row with the operation identity

#### Scenario: Cross-origin POST refused

- **WHEN** a POST arrives at `/ui/projects/{id}/publish` with an `Origin` header that does not match the loopback bind address
- **THEN** the response is 403 HTML and the journal gains no row

### Requirement: Dependency closure explicitly registered

`maud`, `maud_macros`, and `itoa` SHALL be the only additions
to the workspace dependency closure for this package, and all
three SHALL be registered in `deny.toml` with an explicit
reason per entry. No other transitive crate is added.

#### Scenario: `cargo deny check` green

- **WHEN** `cargo deny check` runs
- **THEN** the only new allow-list entries since the pre-change baseline are the three named above, each carrying its reason, and the rest of the closure is unchanged

### Requirement: Browser OIDC sign-in without URL credentials

Forge SHALL provide a browser sign-in flow for the server-rendered portal using a registered project's identity configuration and standard OIDC authorization-code flow with PKCE. Browser routes SHALL never accept or emit a session credential in a query string or HTML form field. An unauthenticated HTML request to `/ui` SHALL redirect to the local sign-in entry page. Opening `/ui/sign-in` without a project parameter SHALL render an accessible form that asks for a project id and starts the existing project-scoped OIDC flow when submitted. The entry page SHALL NOT disclose the registered project inventory.

#### Scenario: Unauthenticated browser reaches sign-in

- **WHEN** a browser requests `/ui` without a session
- **THEN** Forge redirects locally to `/ui/sign-in` and does not render `api-unauthorized`

#### Scenario: Sign-in entry asks for a project

- **WHEN** an unauthenticated browser opens `/ui/sign-in` without a project parameter
- **THEN** Forge renders an accessible required project-id form that submits to the existing sign-in route

#### Scenario: Sign-in starts for a configured project

- **WHEN** an unauthenticated browser opens `/ui/sign-in` with a registered project and a safe local return path
- **THEN** Forge creates a bounded, persisted, project-scoped state/nonce/PKCE challenge and redirects to the configured provider with the configured client, redirect URI, and scopes

#### Scenario: No configured identity is available

- **WHEN** the selected project is unknown, has no identity configuration, or has invalid identity configuration
- **THEN** Forge refuses sign-in with a safe HTML error, performs no provider redirect, and creates no session

#### Scenario: Unsafe return path

- **WHEN** sign-in receives an absolute URL, protocol-relative URL, malformed path, or a path outside `/ui`
- **THEN** Forge uses `/ui` as the post-login destination and never redirects off-origin

#### Scenario: JSON and authenticated callers remain compatible

- **WHEN** an authenticated browser or JSON API client uses existing routes
- **THEN** authenticated portal rendering and `/v1` JSON response contracts remain unchanged

### Requirement: Verified callback and one-use challenge

Forge SHALL consume callback state once and SHALL mint a browser session only after PKCE code exchange and cryptographic OIDC validation of issuer, audience, signature, expiry, nonce, and configured admin claim.

#### Scenario: Valid administrator callback

- **WHEN** the callback matches an unexpired challenge and the provider returns a valid signed ID token with the configured admin claim
- **THEN** Forge persists one active session for that project's identity and redirects to the validated local return path

#### Scenario: Invalid callback or non-admin user

- **WHEN** callback state is missing, mismatched, expired, or replayed, code exchange fails, any ID-token validation fails, or the admin claim is absent
- **THEN** Forge refuses authentication, creates no session, consumes any callback-reached challenge, and returns only a redacted error

#### Scenario: Provider unavailable

- **WHEN** discovery, JWKS retrieval, or code exchange times out or is unavailable
- **THEN** Forge fails closed within the configured request bound and does not mint a session

### Requirement: Project-scoped browser session cookie

Forge SHALL transport the existing opaque project session ID through a host-only HttpOnly SameSite cookie scoped to `/ui`, validate it against current persisted session state on every protected UI request, and preserve bearer-only authentication for `/v1` routes.

#### Scenario: Active session accesses portal

- **WHEN** a browser presents a valid active cookie on a protected `/ui` route
- **THEN** Forge resolves and authorizes that project session using the same owner, expiry, revocation, and permission checks used by the API

#### Scenario: Invalid, expired, revoked, or cross-project cookie

- **WHEN** a cookie is malformed, unknown, expired, revoked, or owned by a different project than the requested project operation
- **THEN** Forge denies access, creates no new session, and clears an expired/invalid browser cookie

#### Scenario: Cookie does not authorize JSON API

- **WHEN** a request to `/v1` supplies only the browser cookie and no Authorization bearer header
- **THEN** the API returns its existing unauthorized response without changing its JSON contract

#### Scenario: Credential supplied in URL

- **WHEN** a browser UI request supplies the legacy `?token=` parameter
- **THEN** Forge refuses or ignores it without reflecting or logging its value; it never authenticates from the query parameter

### Requirement: Browser sign-out and request integrity

Forge SHALL provide same-origin sign-out that revokes/removes only the current project's session, clears the browser cookie, and requires exact same-origin checks for browser state-changing requests.

#### Scenario: Sign out

- **WHEN** an authenticated browser posts sign-out from the portal's exact origin
- **THEN** Forge invalidates only the owning project's current session, expires the cookie, and redirects to the sign-in page

#### Scenario: Cross-origin state-changing request

- **WHEN** sign-out or another state-changing portal request has a missing or mismatched Origin
- **THEN** Forge refuses it before changing session or project state

#### Scenario: Sign-out isolation

- **WHEN** one project session is signed out
- **THEN** sessions belonging to other projects remain governed by their own persisted state and are not implicitly revoked

### Requirement: Responsive portal reflow

Forge SHALL render every portal page so the content reflows at 320 CSS pixels and 400% zoom without page-level horizontal scrolling, except for bounded two-dimensional data tables that may scroll within their own named region.

#### Scenario: Narrow viewport

- **WHEN** an operator views fleet, project detail, portfolio, publish plan/result, error, or Studio pages at 320 CSS pixels
- **THEN** navigation and metadata reflow, controls remain visible, and the document has no horizontal overflow; any wide table scrolls only inside its labelled table region

#### Scenario: Zoom and long content

- **WHEN** text is enlarged to 200% or the page is viewed at 400% zoom with long project identifiers, URLs, or notes
- **THEN** text and controls remain available without overlap, clipping, or loss of content

### Requirement: Semantic structure and keyboard operation

Forge SHALL give every portal document a language, landmark structure, skip link, unique main target, logical heading order, labelled controls, named data tables, and visible keyboard focus. Native semantic HTML SHALL be preferred over redundant ARIA roles.

#### Scenario: Keyboard-only operation

- **WHEN** a user navigates any portal page using only a keyboard
- **THEN** the skip link, navigation, filters, project links, table region, and mutation controls are reachable in reading order and every focused item has a visible, unobscured indicator

#### Scenario: Form and table semantics

- **WHEN** assistive technology inspects a portal form or data table
- **THEN** each input has an associated label, each table exposes a name/caption and scoped headers, and each status is conveyed as text rather than color alone

### Requirement: Portal visual contrast and interaction targets

Forge SHALL meet WCAG 2.2 AA contrast requirements for rendered text and interactive boundaries/focus indicators, provide pointer targets of at least 24 by 24 CSS pixels unless a WCAG exception applies, and honor reduced-motion preference.

#### Scenario: Light and dark presentation

- **WHEN** any portal page renders in light or dark system color scheme, including default, hover, and keyboard-focus states
- **THEN** measured normal-text contrast is at least 4.5:1, large text and non-text UI/focus contrast is at least 3:1, and status remains understandable without color

#### Scenario: Reduced motion and target size

- **WHEN** a user enables reduced motion or uses pointer/touch input
- **THEN** nonessential motion is removed and interactive targets meet the minimum target size without clipping or overlap

### Requirement: Portal accessibility verification

Forge SHALL verify representative rendered portal page families at narrow and desktop viewports with browser automation and record measured contrast and keyboard/accessibility-tree evidence; markup-only tests SHALL NOT be reported as full accessibility verification.

#### Scenario: Browser evidence covers all page families

- **WHEN** the portal UI verification suite runs
- **THEN** it exercises fleet, detail, portfolio, publish plan/result, error, and Studio pages for reflow, keyboard focus, semantic names, and light/dark contrast, and reports each failed page/state

