# Capability: portal-web-ui

## ADDED Requirements

### Requirement: Browser project list with live health
Forge SHALL serve `GET /ui` from the existing API service as framework-free HTML listing every roster project with its latest publish state and its current liveness verdict, reusing API authentication, portal read models, and the fleet liveness evaluation.

#### Scenario: List renders the fleet
- **WHEN** an authorized operator opens `/ui` with `Accept: text/html`
- **THEN** one row per `compose_ready` roster project shows publish state, online verdict, and subdomain link, plus the skipped-entry section, with project-controlled strings escaped

#### Scenario: JSON unchanged
- **WHEN** the same paths are requested with `Accept: application/json`
- **THEN** the pre-change JSON envelopes are byte-identical

### Requirement: Project detail with evidence
Forge SHALL serve `GET /ui/projects/{id}` rendering identity, manifest, maturity, doctor summary, journal rows, and liveness for one project, and SHALL render the portal 404 page for unknown ids.

#### Scenario: Unknown project
- **WHEN** a browser requests `/ui/projects/no-such-app`
- **THEN** the service answers 404 HTML without leaking JSON internals

### Requirement: Confirm-gated republish from the browser
Forge SHALL serve `POST /ui/projects/{id}/publish` such that without confirmation it renders the dry-run plan page and enqueues nothing, and with confirmation it enqueues the Core publish lane and renders the operation identity for tracking, honoring idempotency keys and Core preconditions exactly like the CLI.

#### Scenario: Republish without confirm enqueues nothing
- **WHEN** the confirm form is submitted without `confirm=yes`
- **THEN** the journal gains no row and the response shows the typed refusal or plan

#### Scenario: Confirmed republish tracks
- **WHEN** the confirm form is submitted with `confirm=yes`
- **THEN** the response is 202 with the operation identity and the journal gains the matching row
