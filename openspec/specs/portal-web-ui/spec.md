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

### Requirement: Scoped form fields carry visible labels and helper text

Fleet filter inputs, delivery action inputs, and portfolio metadata
inputs SHALL each carry a visible label that persists while typing
(placeholders stay as examples only), plus helper text where the field
needs explanation. Relabelled controls SHALL keep the slice-2 44px
targets and overall layout. The browser SHALL still send only validated
single-segment names and typed fields to the existing endpoints; no new
endpoint SHALL be called.

#### Scenario: Read a fleet filter with its guidance visible

- **WHEN** the operator views the fleet filter row
- **THEN** each of the six inputs shows a persistent visible label and
  the row shows helper text explaining blank-means-any predicate
  matching

#### Scenario: Fill a delivery or portfolio field

- **WHEN** the operator fills a delivery allowlist/publish/reconcile/
  lookup field or a portfolio tag/review field
- **THEN** the field shows a visible label and helper text naming the
  accepted value, and the submitted payload shape is unchanged

### Requirement: Failed submissions show a focusable error summary with per-field links

Every failed submission on a scoped form (login, portfolio, delivery,
workbench action, workspace/management onboarding) SHALL render a
focusable error summary (heading plus one link per failing field),
move focus to it, retain the inline field errors, and wire each inline
error to its field with `aria-describedby` (plus `aria-invalid` while
the error shows). The fleet filter error SHALL use `role="alert"`.

#### Scenario: Submit with a missing value

- **WHEN** the operator submits a scoped form with a missing or refused
  value
- **THEN** focus moves to the summary, each summary link targets its
  field, each field keeps its inline error, and the error id is present
  in the field's `aria-describedby` until cleared

#### Scenario: Catalog predicate read fails

- **WHEN** the fleet filter predicate read fails
- **THEN** the row-level error is exposed as `role="alert"` while the
  full table keeps showing

### Requirement: Login offers a password toggle and visible required indicators

The login form SHALL offer a show/hide password toggle (native button,
`aria-pressed`, labelled Show/Hide) and SHALL mark required fields with
a visible indicator plus a legend. `name`, `autocomplete`, and paste
behavior SHALL stay intact so password managers keep working.

#### Scenario: Reveal the typed password

- **WHEN** the operator activates the toggle
- **THEN** the password field switches between hidden and readable, the
  toggle label and `aria-pressed` follow, and the typed value is
  preserved

#### Scenario: Sign in with a password manager

- **WHEN** the operator fills login via a password manager or paste
- **THEN** the fields accept the fill (unchanged `name`/`autocomplete`,
  no paste blocking) and failed sign-in still reports the generic error
  through the summary pattern

### Requirement: Responsive breakpoints cover small phones through wide desktop plus short landscape

The dashboard stylesheet SHALL systematize responsive behavior across
the 375 / 768 / 1024 / 1440 steps plus short-landscape orientation:
content tightening at or below 1024px, the existing 850px rule serving
the 768 tablet step, small-phone compaction at 375px, and hero
compaction gated on landscape orientation with short viewport height.
Desktop layout above 1024px SHALL keep its current feel with no
computed-value change.

#### Scenario: Open any dashboard route at 375px wide

- **WHEN** the operator views the login or any dashboard route at 375
  CSS pixels
- **THEN** content, sidebar, topbar, and cards compact with no
  page-level horizontal scroll and every control stays operable

#### Scenario: Open the portal in short landscape

- **WHEN** the operator views the portal in landscape orientation with
  a short viewport height
- **THEN** the auth hero compacts so the sign-in form stays reachable,
  while desktop landscape keeps its full layout

#### Scenario: Desktop above 1024px is unchanged

- **WHEN** the operator views any route on a desktop viewport wider
  than 1024px
- **THEN** layout, spacing, and type match the pre-change design

### Requirement: Small-phone sidebar keeps every destination reachable without page scroll

At and below the tablet step the sidebar SHALL present its 5
destinations in a scrollable in-row nav region (scrolling inside the
nav, never the page), wrapping brand-above/nav-below at 375px. Every
destination SHALL stay reachable and keyboard-focusable, the active
destination SHALL keep its visible active state plus `aria-current`,
and nav targets SHALL keep their 44px minima.

#### Scenario: Reach every destination at 375px

- **WHEN** the operator opens the dashboard at 375 CSS pixels
- **THEN** all 5 nav links are reachable with no horizontal page
  scroll, and the active link is visibly marked

### Requirement: Filter and search state survives reload, back/forward, and view switches

The fleet search box, source filter, and the six catalog predicate
inputs SHALL have their typed values preserved across dashboard boot
(reload), back/forward traversal, and switches back into the projects
view, re-applying the fleet render path on restore. The URL SHALL
remain the source of truth for `?project=` deep links: project
selection is never snapshotted and never restored from storage.

#### Scenario: Reload with fleet filters typed

- **WHEN** the operator types fleet search/filter values and reloads
- **THEN** the values are restored and the table re-filters as before

#### Scenario: Leave and return to the projects view

- **WHEN** the operator switches to another view and back
- **THEN** the typed search/filter values are still present and applied

#### Scenario: Deep links stay URL-owned

- **WHEN** the operator follows `/workbench?project=<id>` or
  `/management?project=<id>` through reload or back/forward
- **THEN** the URL's project boots exactly as before, unaffected by
  any preserved filter state

### Requirement: Unknown dashboard paths render an honest empty state

A client-side pathname outside the dashboard route allowlist SHALL
render an honest empty state naming the miss and linking the 5 real
destinations, instead of silently showing the fleet. No nav link SHALL
claim the active state there, and the crumb/title SHALL name the
missed page.

#### Scenario: Open an unknown path

- **WHEN** the dashboard router sees a pathname outside the route
  allowlist
- **THEN** the unknown section shows with an explanation and the 5
  destination links, and no sidebar link is marked active

### Requirement: Overlays, nav, and dropdowns layer on a named z-index scale

The stylesheet SHALL define a layered z-index scale ordering sticky
chrome, nav, dropdown wrappers, banners, overlays, and the skip link,
with each existing layered element assigned to its rung. The skip link
SHALL remain the topmost layer.

#### Scenario: Inspect the stacking order

- **WHEN** the shipped stylesheet is inspected
- **THEN** a named scale assigns every layered element to its rung
  with the skip link topmost, and sticky/nav/dropdown layers stack
  predictably

### Requirement: Portal icons are inline SVG from one stroke set

Every UI glyph SHALL be an inline SVG from a single consistent stroke
set (24 viewBox, `currentColor`, round caps/joins, uniform
stroke-width token, sm/md/lg size tokens) instead of a unicode glyph.
Decorative icons beside visible text SHALL carry `aria-hidden`;
accessible names on labelled controls SHALL stay intact; no emoji
SHALL appear anywhere in the portal.

#### Scenario: Scan nav, search, summary, and empty states

- **WHEN** the operator views any dashboard route or the login page
- **THEN** sidebar, search, summary-card, empty-state, story-check,
  lock, submit, and disclosure icons render from the one stroke set
  at uniform weight and aligned sizes, with no unicode-glyph icon
  remaining

#### Scenario: Assistive technology meets an icon

- **WHEN** a screen reader encounters a decorative icon beside visible
  text
- **THEN** the icon is hidden (`aria-hidden`) and the visible label
  (or `sr-only`/`aria-label` name on labelled controls) announces as
  before

### Requirement: Portal type declares its base, scale, figures, and prose guard

The stylesheet SHALL declare an explicit 16px base with body
line-height in the 1.5–1.75 band, a named type scale, a 65–75ch
line-length guard on prose, tabular figures for counts/ids/timestamps,
and long-token wrapping via `overflow-wrap:anywhere` (never
`word-break:break-all`). Every normal-text pair SHALL keep meeting
4.5:1.

#### Scenario: Read body copy and figures

- **WHEN** the operator reads dashboard or login copy at the default
  size
- **THEN** body text renders at 16px with 1.6 line-height, prose lines
  wrap at or under 70ch, and counts/ids/digests/timestamps render in
  tabular figures

#### Scenario: A long token meets a narrow card

- **WHEN** a digest or token exceeds its container width
- **THEN** it wraps sanely (`overflow-wrap:anywhere`) without clipping
  or breaking the layout

#### Scenario: Contrast after the type pass

- **WHEN** any portal text/background pair is measured
- **THEN** normal-text contrast still meets 4.5:1 (no color token
  changed)

### Requirement: Portal motion runs on shared enter/exit tokens

Interactive motion SHALL use shared duration/easing tokens with exits
at 60–70% of enters (no one-duration-everywhere), SHALL stay
transform/opacity-only, SHALL keep the `prefers-reduced-motion` guard,
and SHALL animate no layout property.

#### Scenario: Hover and press a control

- **WHEN** the operator hovers (enter) and presses (exit) a button,
  nav link, or action head
- **THEN** the enter runs at the shared enter duration and the press
  confirms at the faster shared exit duration, with no sibling
  movement

#### Scenario: Reduced-motion operator uses the portal

- **WHEN** reduced motion is preferred
- **THEN** all transitions/animations collapse as before and
  programmatic scrolls stay instant

### Requirement: Fleet table columns sort with an announced sort state

The fleet `Project`, `Details`, and `Status` columns SHALL be sortable
through keyboard-reachable header controls that expose the current sort
state with `aria-sort` on the header cell. Sorting SHALL be stable and
SHALL compose with the existing free-text, source, and catalog-predicate
filters (filter code, the catalog `limit:1000` query, and the JSON shape
stay unchanged). The `Open` action column SHALL stay unsorted.

#### Scenario: Sort the fleet by a column

- **WHEN** the operator activates the `Project` header control
- **THEN** rows order by project name (stable for ties), the header
  cell reports `aria-sort="ascending"`, and activating again reverses
  to `descending`

#### Scenario: Sort composes with filters

- **WHEN** free-text, source, or catalog-predicate filters are active
  and a sort is applied
- **THEN** the table shows the filtered set in the requested order —
  never unfiltered rows, never a filter reset

#### Scenario: Keyboard and screen-reader operation

- **WHEN** the operator tabs to a fleet header control or inspects it
  with assistive technology
- **THEN** the control is reachable by keyboard, labelled
  `Sort by <column>`, and its header cell announces the current sort
  state via `aria-sort`

### Requirement: Filtered fleet rows export as CSV in the browser

The dashboard SHALL offer an `Export filtered CSV` control that
downloads the currently filtered and sorted fleet rows (pre-page) as
an RFC-4180-escaped CSV through a plain browser download. No new
endpoint SHALL be called and no server state SHALL be created.

#### Scenario: Export the current view

- **WHEN** the operator activates `Export filtered CSV` with filters
  and a sort applied
- **THEN** the downloaded file contains a header row plus one row per
  filtered project in the displayed order, with commas, quotes, and
  newlines escaped

#### Scenario: Empty filtered set

- **WHEN** no project matches the current filters
- **THEN** the export control is disabled and no download is offered

### Requirement: Large fleets render windowed with honest counts

The fleet table SHALL mount at most one page of rows
(`FLEET_PAGE_SIZE = 50`) and SHALL report the honest window
(`Showing X of Y filtered (Z total)`). Paging SHALL apply after
filtering and sorting so filter composition stays exact, and any
filter or sort change SHALL reset to the first page.

#### Scenario: Browse a large filtered fleet

- **WHEN** the filtered set exceeds one page
- **THEN** only the current page's rows are mounted, the count names
  the window and the filtered/total figures, and Previous/Next move
  the window without changing the set

#### Scenario: Shrink the set while paged

- **WHEN** the operator is past page one and narrows the filters so
  fewer pages remain
- **THEN** the view clamps to a valid page (first page on filter or
  sort change) and the count stays exact

### Requirement: Async regions show skeleton placeholders and reserve count space

While the fleet fetch is in flight the table region SHALL show
skeleton/shimmer placeholder rows (not text alone) with
`aria-busy="true"`, and delivery/maintain loaders SHALL use the same
skeleton treatment. Summary counts SHALL reserve space so their
arrival does not shift layout. Shimmer SHALL collapse under
`prefers-reduced-motion` and SHALL animate no layout property.

#### Scenario: Load the dashboard on a slow connection

- **WHEN** `/v1/admin/projects` has not yet resolved
- **THEN** the fleet region shows placeholder rows with `aria-busy`,
  and summary counts keep their reserved space instead of reflowing
  on arrival

#### Scenario: Reduced-motion operator loads the dashboard

- **WHEN** reduced motion is preferred
- **THEN** placeholders render statically with no shimmer animation

### Requirement: Portal action results are announced as live regions

Every portal action result region (workbench plan/apply, delivery
action and lookup, workspace-bulk preview/run, scoped-management
preview/run, and each catalog-action card's preview/refusal/success)
SHALL be a live region. A successful, preview, or informational result
SHALL announce politely (`role="status"`); a refusal SHALL announce
assertively (`role="alert"`). The role SHALL be set on every write so a
region reused for a later outcome never keeps the previous urgency, and
the existing visible result markup SHALL be retained.

#### Scenario: Screen reader hears a completed action

- **WHEN** a screen-reader operator runs a confirmed portal action and
  the operation succeeds
- **THEN** the result region is a polite live region and its success text
  is announced

#### Scenario: Screen reader hears a refusal

- **WHEN** a portal action, preview or onboarding run is refused
- **THEN** its result region is an assertive live region and the refusal
  text is announced, alongside the existing focusable error summary

#### Scenario: A reused result region does not keep stale urgency

- **WHEN** a result region previously showed a refusal and is then
  reused for a successful action
- **THEN** the region announces politely for the success

### Requirement: An expired mid-session credential returns to sign-in

When a protected Forge admin request made from a dashboard page returns
`HTTP 401`, the frontend SHALL return the operator to the sign-in page
with the current same-origin path and query as the `next` deep link,
instead of showing a generic failure with no recovery. The login page
SHALL NOT trigger this redirect, so a rejected credential on the login
page still renders its inline error and summary.

#### Scenario: Session expires during dashboard use

- **WHEN** the operator's Forge-wide session expires or is revoked while
  the dashboard is open and the next admin request returns `401`
- **THEN** the browser navigates to `login.html?next=<current path and
  query>` and no dashboard action control is left claiming success

#### Scenario: Wrong password on the login page

- **WHEN** the operator submits incorrect credentials on the login page
  and the API returns `401`
- **THEN** no redirect occurs and the inline field error and error
  summary render as before

#### Scenario: Return path stays same-origin

- **WHEN** the session-expiry redirect is followed after sign-in
- **THEN** the `next` value is validated as a same-origin dashboard route
  before navigation and an off-origin or malformed value is dropped

### Requirement: Essential portal text meets a 12px floor

Portal text that carries content an operator must read — error text,
helper/hint text, evidence chips, digests, action CLI hints, findings,
workflow reasons, source metadata and detail rows — SHALL render at no
less than 12 CSS pixels. Uppercase micro-labels and badges MAY remain
smaller. The change SHALL NOT reduce contrast or alter the declared base
size.

#### Scenario: Read an error or digest

- **WHEN** the operator reads a field error, a validation hint, an
  evidence chip, a digest or a workbench action's CLI hint
- **THEN** the text renders at 12px or larger

#### Scenario: Prior sizes and tokens survive

- **WHEN** the shipped stylesheet is inspected after this change
- **THEN** the base `16px` declaration, the WCAG contrast pairs and the
  existing `--text-*` tokens are unchanged, and the 44px touch minima are
  intact

### Requirement: Workbench disclosure controls name the panel they control

Each workbench catalog-action disclosure button SHALL reference the
panel it expands with `aria-controls`, in addition to its existing
`aria-expanded` state.

#### Scenario: Assistive technology inspects a disclosure button

- **WHEN** assistive technology or an accessibility audit inspects a
  workbench action's disclosure button
- **THEN** the button carries `aria-expanded` and an `aria-controls`
  value that matches the id of the `.wb-action-body` it discloses

### Requirement: Lifecycle series rail in the workbench

The workbench lifecycle area SHALL render an ordered series rail with
the 8 steps Idea, Scaffold, Spec, Code, Test, Release, Deploy and
Operate, derived from the existing doctor, status, delivery and
journal projections, with exactly one step carrying
`aria-current="step"` and every step deep-linking with both
`?project=` and `?step=`.

#### Scenario: Eight derived steps with a single current

- **WHEN** a managed project is open in the workbench
- **THEN** `ol#lifecycle-rail` lists the 8 steps in series order,
  each state derived from manifest maturity/target, doctor health,
  status checks, delivery phase/verbs/next and journal operation
  kinds, with the first incomplete step (or Operate when all are
  done) carrying `aria-current="step"` and no step inventing
  evidence its projection did not supply

#### Scenario: Step deep link preserves the project

- **WHEN** the operator follows or reloads a rail step link
- **THEN** the URL carries both `?project=<id>` and `?step=<key>`,
  project switches preserve a valid step, step clicks preserve the
  project, applying the step scrolls to the mapped card without
  moving `aria-current`, and unknown step values are ignored

### Requirement: Next-best-action card in the workbench

The workbench SHALL render a single next-best-action card computed
from maturity, target and evidence, naming the one action, the
reason, and the exact CLI.

#### Scenario: One ranked Next with reason and CLI

- **WHEN** the workbench projections resolve
- **THEN** the next-best-action card shows the single ranked pick
  (declare maturity, run doctor, pick target, unblock delivery,
  mapped delivery action, plan upgrade, or sustain watch) with a
  human reason, the exact CLI when the mapped catalog command is
  known, and a control opening the matching action card

### Requirement: Copy-as-CLI on every confirm action

Every catalog-driven confirm action SHALL carry a Copy-as-CLI button
rendering the exact `forge ...` string built from the catalog
`cli_invocation`, the project positional for scoped rows, and the
card's own typed payload, with secrets redacted.

#### Scenario: Full string shown and copied

- **WHEN** the operator opens any confirm action and activates Copy-as-CLI
- **THEN** the full untruncated `forge ...` string renders in the
  card, copies via the clipboard with a select-the-text fallback,
  and secret-ish fields (`confirm*`, `*token*`, `secret_ref`) appear
  only as `<token>`/`<redacted>` placeholders

### Requirement: Observability shortcuts on flagged rows

Failed doctor/status rows, stale fleet rows and unfinished Hermora
verbs SHALL link to their existing remediation surface with the exact
CLI shown.

#### Scenario: Failed check previews its remediate plan

- **WHEN** a doctor finding or status check renders as failed or
  unavailable
- **THEN** its row carries a `Plan remediate` button rendering the
  full `forge remediate plan --finding <id>` string, copying it via
  the clipboard with a select-the-text fallback, and never sending a
  filesystem path

#### Scenario: Stale rows link to reconcile

- **WHEN** a fleet row is conflicted, stale, unavailable or publish-failed
- **THEN** its action cell appends a `Refresh & reconcile` deep link
  to `/management?project=<id>` while healthy rows keep exactly
  their Open/Manage control

#### Scenario: Unfinished Hermora retries inline

- **WHEN** the delivery projection records a Hermora verb that is not done
- **THEN** the delivery card shows a `Retry Hermora` control opening
  the existing `delivery.hermora-retry` action card beside the exact
  CLI string

### Requirement: Rail keyboard and mobile hardening

The series rail SHALL be keyboard-operable with text alternatives,
and the shell SHALL stay overflow-free at 390px with sticky headers.

#### Scenario: Roving rail with named rows

- **WHEN** the operator tabs to the rail and uses arrows/Home/End
- **THEN** exactly one rail link is in the Tab order, focus moves
  within the rail, and every row announces `<Label>:
  <done|current step|upcoming>` with a single `aria-current="step"`

#### Scenario: 390px shell with sticky context

- **WHEN** the workbench renders at 390 CSS px
- **THEN** the 232px sidebar presents as a topbar row under 860px,
  table headers stick inside their scroll regions, panel tool
  clusters wrap, and the page has no horizontal overflow

### Requirement: Lifecycle rail browser oracle

The change SHALL ship a live Chromium oracle proving rail shape,
coexistence, copy exactness, roving, overflow and contrast.

#### Scenario: Pinned harness verifies the rail end to end

- **WHEN** `tests/lifecycle_rail_browser.rs` runs with the pinned
  `tests/browser` playwright 1.63.0 and Chromium available
- **THEN** `lifecycle-rail-check.mjs` verifies 8 ordered steps, one
  current, one Tab stop, step/project coexistence across
  load/click/reload, bogus-step tolerance, one Next, arrow roving,
  clipboard-equals-shown copy, 390px no overflow and contrast AA;
  without the toolchain it reports UNVERIFIED (exit 2), never a pass

### Requirement: Idea entry in workbench rail step zero

The workbench lifecycle card SHALL render an idea entry block inside step 0 (Idea) naming the graduation preview/import CLI and a studio spec entry link, preserving `?project=` and `?step=` deep links, with no new endpoint or framework.

#### Scenario: Idea entry names graduation and studio

- **WHEN** the workbench loads a managed project
- **THEN** `#wb-idea-entry` names `forge graduation preview`, `forge graduation import --confirm`, and links to the studio spec entry (`forge studio spec` + `/workbench?project=<id>&step=spec`)

#### Scenario: Idea links preserve project and step

- **WHEN** the operator follows an idea entry link
- **THEN** the URL carries both `?project=<id>` and `&step=` and an unknown `&step=bogus` is still ignored without moving `aria-current`

### Requirement: Publish to maintain loop in delivery

A successful delivery publish SHALL render a Next-idea prompt with a workbench `&step=idea` link and a maintain refresh shortcut that refreshes the workbench maintain view, reusing the existing maintain refresh control.

#### Scenario: Publish success writes the Next idea prompt

- **WHEN** `Publish approved preview` succeeds
- **THEN** `#delivery-next-idea` names the Next idea prompt, links to `/workbench?project=<id>&step=idea`, and offers a maintain refresh shortcut firing `#wb-maintain-refresh`

#### Scenario: Publish failure writes no loop

- **WHEN** the publish is refused or fails
- **THEN** no Next-idea block renders and the existing error summary receives focus

### Requirement: Capability group filter and rail badge

The projects view SHALL offer a capability group filter and the workbench rail SHALL carry a capability badge, both as text (never color-only) with no new frontend dependency.

#### Scenario: Projects filter by cap group

- **WHEN** the operator picks a group in `#cap-filter`
- **THEN** the fleet table shows only projects in that `cap_group` and the count line names the active group

#### Scenario: Rail carries a cap badge

- **WHEN** the workbench loads a managed project
- **THEN** `#cap-badge` names the project's capability group in words with `role="status"`

### Requirement: Canonical flywheel demo script

The repository SHALL ship `docs/flywheel-demo.md` walking the `hookit` candidate through idea→scaffold→gate→publish→maintain with exact CLI strings and `?project=&step=` web URLs.

#### Scenario: Demo covers five steps with URLs

- **WHEN** the operator opens `docs/flywheel-demo.md`
- **THEN** five steps (idea, scaffold, gate, publish, maintain) each name the exact `forge …` command and a `/workbench?project=hookit&step=<key>` URL

### Requirement: GitHub metadata views in portal and web

The repositories portal controls and the Delivery web view SHALL expose GitHub metadata observe/propose/create guidance with an accessible topics list, a propose form with a confirm-token field, and a create flow with visibility radios and a push-source checkbox, reusing existing portal/web patterns with no new framework.

#### Scenario: Topics list with dev highlight and text alternative

- **WHEN** the Delivery GitHub metadata card renders topics
- **THEN** the topics appear as a real `<ul>` with dev-prefixed topics (`dev-`/`forge-`, case-insensitive) highlighted in addition to text, and a text alternative names the count and values

#### Scenario: Propose form carries a confirm token field

- **WHEN** the operator opens the propose form
- **THEN** repository, field, value, pull-request/direct mode radios, and a masked confirm-token field are labelled, keyboard operable, validated with an error summary receiving focus, and results announce via a `role="status"` region

#### Scenario: Create flow carries visibility radios and push-source checkbox

- **WHEN** the operator opens the create flow
- **THEN** project, repository, private/public visibility radios, push-source and register-if-missing checkboxes, and an explicit action confirm are labelled and keyboard operable, and the previewed CLI string is exact

#### Scenario: Portal names the github commands

- **WHEN** `forge portal view repositories` renders
- **THEN** `controls_available` names the observe, direct topic propose, and create (with `--register-if-missing`) commands

### Requirement: Dashboard exposes a read-only command-reference browser

The dashboard's projects view SHALL carry a read-only command
reference section listing every `GET /v1/admin/commands` catalog row
from the already-fetched catalog JSON, with no new endpoint, no new
fetch and no new frontend dependency. Every row SHALL show its
availability badge, summary and exact `forge ...` CLI string; every
non-web row (`cli_only`, `not_yet_web`, `provider_required`,
`project_capability_required`, `disabled`) SHALL additionally show its
plain-language reason and a clipboard Copy button for the CLI string,
and SHALL never render an executable control. Every `web` row SHALL
link to the existing view that already serves it (`/projects`,
`/workbench`, `/management`, `/portfolio` or `/delivery`).

#### Scenario: Operator discovers what the browser cannot do

- **WHEN** the operator opens the projects view with the catalog loaded
- **THEN** the reference lists every catalog row, each non-web row
  shows its availability badge, plain-language reason and exact CLI
  string with a Copy button, and no non-web row offers an executable
  control

#### Scenario: Operator follows a web row to its view

- **WHEN** the operator activates a web row's link
- **THEN** the browser navigates to the existing view serving that
  command through the standard router (deep-link safe)

#### Scenario: Catalog unavailable

- **WHEN** the catalog fetch fails or returns no rows
- **THEN** the reference renders an honest unavailable state instead
  of an empty list or a stale copy

### Requirement: Command reference is searchable, filterable and accessible

The reference SHALL offer a text search (matching id, label, summary,
CLI string, reason and category) and an availability filter covering
all six catalog states, combined as AND, with a live result count and
an honest no-matches state. Controls SHALL be native keyboard-operable
elements with visible focus; the count and copy feedback SHALL be
`role="status"` live regions; essential text SHALL meet the 12px floor;
no new motion SHALL be introduced; all catalog strings SHALL be
rendered as text (never interpreted as HTML); and no shell, path or
command submission surface SHALL be added.

#### Scenario: Operator narrows the reference

- **WHEN** the operator types in the search box or picks an
  availability state
- **THEN** only matching rows render and the live count announces the
  narrowed result

#### Scenario: Screen-reader operator copies a CLI string

- **WHEN** the operator activates a row's Copy button
- **THEN** the exact CLI string is written to the clipboard (or a
  select-the-text fallback is offered) and the copy live region
  announces the outcome

