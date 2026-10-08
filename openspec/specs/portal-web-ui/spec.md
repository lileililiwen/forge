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

### Requirement: Standalone frontend assets

Forge SHALL keep browser HTML, CSS and JavaScript in a standalone `frontend/` directory served independently of Rust API routes. The Rust API SHALL expose JSON contracts only for this workflow and SHALL NOT render, embed, or generate the login or dashboard markup, styles, or scripts.

#### Scenario: Frontend assets have an independent owner

- **WHEN** the operator starts the local frontend preview
- **THEN** the login and dashboard load from the `frontend/` assets without requesting an HTML document from an API route
- **AND** the Rust API supplies authentication and project data as JSON

### Requirement: Forge-wide login experience

The standalone Forge frontend SHALL provide an accessible email/password login inspired visually by the supplied AllTools page. It SHALL NOT ask for a project id, show nonfunctional social-login controls, or place credentials in a URL. Anonymous session state SHALL show login; an uninitialized Forge administrator SHALL show the exact `forge identity setup --email <address>` setup instruction without fleet data.

#### Scenario: Anonymous operator opens the frontend

- **WHEN** the frontend finds no valid Forge-wide session
- **THEN** it shows email and password controls and no project-id control
- **AND** invalid credentials produce a generic error without revealing whether the email exists

#### Scenario: Forge admin has not been initialized

- **WHEN** the frontend receives the uninitialized state from the admin session API
- **THEN** it displays the local CLI setup instruction and does not request or show project data

### Requirement: Authenticated all-project dashboard

The standalone frontend SHALL show a Forge-wide dashboard for every registered project after authentication. Summary counts, project rows and recent operation evidence SHALL come from the authenticated API and registry/journal; empty, unavailable and stale data SHALL be represented honestly. Project identifiers MAY appear in authenticated project rows and links.

#### Scenario: Authenticated operator opens the dashboard

- **WHEN** the frontend receives an authenticated session state
- **THEN** it requests and displays every registered project without requiring per-project identity configuration
- **AND** it supports searching/filtering projects and signing out

#### Scenario: No projects are registered

- **WHEN** the authenticated fleet response contains no projects
- **THEN** the dashboard shows an explicit empty state and a useful Forge next action without fabricated metrics

### Requirement: Frontend and API boundary security

Credentialed cross-origin requests SHALL be accepted only from the configured frontend origin. The frontend SHALL send the Forge session cookie through credentialed requests and SHALL NOT store passwords or session tokens in browser storage. State-changing requests SHALL validate the frontend Origin. The frontend SHALL remain readable and operable by keyboard, visibly focused, responsive at 320 CSS pixels, and respect reduced motion.

#### Scenario: Untrusted frontend origin calls the API

- **WHEN** a browser sends a credentialed admin request from an origin other than the configured frontend origin
- **THEN** the API rejects the cross-origin request and does not disclose session or fleet data

#### Scenario: Operator uses the frontend on a narrow viewport

- **WHEN** the operator navigates the login and dashboard at 320 CSS pixels using keyboard controls
- **THEN** the interface remains navigable, readable and free of clipped controls, with visible focus and responsive navigation

### Requirement: Focus moves to the main content region on dashboard route change

After the dashboard router switches views, keyboard and screen-reader
focus SHALL move to the `#main-content` region. Same-view parameter
reconciliations (reload, back/forward across `?project=` values, fleet row
actions that stay on the view) SHALL NOT move focus. The move SHALL NOT
alter the URL, history entries, scroll restoration, or the deep-link
parameter handling owned by `fleet-manage-deep-link`.

#### Scenario: Sidebar navigation announces the new view

- **WHEN** the operator activates a sidebar link to a different view
- **THEN** the new view is shown and focus is on the `#main-content`
  region, so assistive technology announces the new view landmark

#### Scenario: Same-view project change keeps focus

- **WHEN** the operator follows a fleet row action that stays on the
  current view with a different `?project=`
- **THEN** the scoped card or workbench detail reconciles as before and
  focus is not moved out of the operator's current control

#### Scenario: Deep-link reload and back/forward survive

- **WHEN** the operator reloads `/workbench?project=<id>` or traverses
  back/forward across two managed ids
- **THEN** the URL's project still boots and no hand selection is
  clobbered by the focus behavior

### Requirement: Keyboard focus is never obscured by the topbar

The sticky topbar SHALL NOT cover a keyboard-focused target. The document
SHALL declare a `scroll-padding-top` offset of at least the topbar height
plus breathing room, and the main content region and titled in-view
sections SHALL carry a matching `scroll-margin-top`, meeting the WCAG 2.2
focus-not-obscured minimum.

#### Scenario: Tab reaches content below the sticky bar

- **WHEN** the operator tabs through the dashboard with the sticky topbar
  visible
- **THEN** every focused target scrolls fully into view below the bar and
  no focused control or heading is hidden behind it

### Requirement: Both auth and dashboard pages pair their color-scheme declaration with the dark token set

The login page and the dashboard shell SHALL declare the same
`color-scheme` value that the shared dark token set implements, and every
normal-text pair in that theme SHALL meet at least 4.5:1 contrast.
Secondary (`--faint`) text on badge-chip backgrounds SHALL meet 4.5:1.

#### Scenario: Login and dashboard agree on the dark theme

- **WHEN** the operator opens `login.html` or any dashboard route
- **THEN** both documents declare the dark scheme and render secondary,
  badge, placeholder, and muted text at 4.5:1 or better in that theme

### Requirement: Login inputs show a visible keyboard focus indicator

Login form inputs SHALL show a focus indicator with at least a 2px
perimeter at 3:1 contrast against the input background in addition to any
border-color change. No input SHALL suppress its outline without providing
that indicator, and every `:focus-visible` rule SHALL resolve to a defined
focus token.

#### Scenario: Tab into the password field

- **WHEN** the operator tabs to a login input
- **THEN** a 2px outline at 3:1 or better is visible around the input
  whether or not the border color also changes

### Requirement: Operator touch targets are at least 44 CSS pixels tall

Every tappable operator control SHALL present a touch target at least
44 CSS pixels tall: `.button`, `.button-quiet`, `.button-primary`,
`.nav-link`, `.filter-box`, `.fleet-filter input`, `.search-box`,
`.login-form input`, workbench text inputs, `#ws-rows` text inputs, and
`.wb-maintain-decide`. Heights SHALL use `min-height` so taller content
still fits, and existing padding/font-size (visual density) SHALL be
unchanged. Controls whose visual must stay small (checkboxes) SHALL keep
their visual size and meet the target through their activating hit area
(wrapping label rows at 44px).

#### Scenario: Tape-measure audit of operator controls

- **WHEN** the shipped stylesheet is inspected for the listed selectors
- **THEN** each computes `min-height >= 44px` (or a 44px activating
  label/row area for checkboxes) with no reduced padding or font-size

#### Scenario: Dense surfaces keep their density

- **WHEN** the operator views the fleet filter row, workspace candidate
  table, or maintain decisions
- **THEN** row padding, font sizes, and column structure match the
  pre-change design; only hit areas grow

### Requirement: Taps respond without delay and confirm on press

Interactive elements SHALL carry `touch-action: manipulation` so taps
never wait on the legacy tap delay. Tappable elements SHALL show press
feedback within 80–150ms via opacity/elevation change that causes no
layout shift, and every clickable SHALL show `cursor:pointer`.

#### Scenario: Tap a control on a touch device

- **WHEN** the operator taps any button, nav link, filter, input, or
  action head
- **THEN** the press is acknowledged within 150ms with no page-level
  double-tap wait and no sibling movement

#### Scenario: Hover a clickable with a pointer

- **WHEN** the operator hovers any button, nav link, maintain decision,
  or checkbox control
- **THEN** a pointer cursor is shown

### Requirement: Fixed and sticky chrome respects safe areas and the dynamic viewport

Sticky/fixed chrome (`.topbar`, `.sidebar`, `.skip-link`) and the page
body SHALL offset with `env(safe-area-inset-*)` (with zero fallbacks) so
tappables never collide with the notch or gesture bar, and both
documents SHALL declare `viewport-fit=cover`. Viewport-filling minima
SHALL track the dynamic viewport (`100dvh`, each keeping its `100vh`
fallback) so sections stay correct as the mobile URL bar shows/hides.

#### Scenario: Open the dashboard on a notched phone

- **WHEN** the operator loads any dashboard route with a notch/gesture
  bar present
- **THEN** the topbar, sidebar, skip link, and page edges clear the
  insets and no tappable sits under the cutout or home indicator

#### Scenario: Scroll the mobile URL bar

- **WHEN** the mobile browser shows or hides its URL bar
- **THEN** the auth layout and app shell keep filling the visible
  viewport with no jump or gap

### Requirement: Programmatic scroll honors reduced motion

The two programmatic smooth scrolls (Maintain decision prefill,
workbench detail open) SHALL scroll instantly when the operator prefers
reduced motion and smoothly otherwise. No unconditional
`{ behavior: "smooth" }` scroll SHALL remain.

#### Scenario: Reduced-motion user opens a workbench project

- **WHEN** reduced motion is preferred and the workbench detail loads
- **THEN** the title scrolls into view instantly with no animation

#### Scenario: Motion-tolerant user jumps to a Maintain decision

- **WHEN** reduced motion is not preferred and the operator opens a
  Maintain decision
- **THEN** the card scrolls into view smoothly, centered as before

