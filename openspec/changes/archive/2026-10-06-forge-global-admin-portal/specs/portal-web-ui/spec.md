# portal-web-ui Specification

## Purpose

Provide a standalone browser frontend for signing in to Forge and managing the complete registered-project fleet.

## Requirements

## ADDED Requirements

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
