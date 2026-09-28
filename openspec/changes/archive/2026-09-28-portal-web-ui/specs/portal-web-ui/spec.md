# Capability: portal-web-ui

## ADDED Requirements

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