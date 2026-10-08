# forge-web-navigation-routing Specification

## Purpose
Serve each dashboard sidebar destination at a real deep-linkable path showing only its own view, backed by an exact-path static allowlist with unchanged auth and security posture.
## Requirements
### Requirement: The static server serves the dashboard shell at deep-linkable paths

`forge web serve` SHALL serve the application shell `index.html` at the
dashboard paths `/projects`, `/workbench`, `/management`, `/portfolio` and
`/delivery`, in addition to the existing `/index.html`. The root path `/` (and
`/login.html`) SHALL continue to serve `login.html`. The server SHALL remain an
exact-path allowlist: any other request path, including a path containing `..`
or an encoded traversal segment, SHALL return 404 and SHALL NOT cause any file
outside the root to be read. The server's existing response headers and
security posture SHALL be unchanged.

#### Scenario: Every dashboard path returns the shell

- **WHEN** a client requests `/projects`, `/workbench`, `/management`,
  `/portfolio`, `/delivery` or `/index.html`
- **THEN** the server responds 200 with `text/html` and the exact bytes of the
  application shell `index.html`

#### Scenario: The login entry stays at root

- **WHEN** a client requests `/`
- **THEN** the server responds 200 with the `login.html` bytes, exactly as
  before this change

#### Scenario: Unknown path is still refused

- **WHEN** a client requests a path that is not on the allowlist
- **THEN** the server responds 404 and reads no file

#### Scenario: Traversal is still refused

- **WHEN** a client requests a path that attempts to escape the root (for
  example `/../src/main.rs` or `/../../etc/passwd`)
- **THEN** the server responds 404 and never returns or reads that file

### Requirement: The dashboard navigates by real path and shows only the active view

`frontend/index.html` SHALL declare `<base href="/">` and SHALL present each
sidebar destination as a real same-origin path link — All projects to
`/projects`, Workbench to `/workbench`, Manage projects to `/management`,
Portfolio to `/portfolio` and Delivery to `/delivery` — with no `#` fragment as
the navigation mechanism. The dashboard SHALL show only the view for the
current URL, SHALL mark the matching sidebar link as the active one with
`aria-current="page"`, and SHALL keep the topbar breadcrumb and document title
in step. Loading a route directly SHALL show that route's view, and browser
Back and Forward SHALL switch views without a full reload. A view whose data
fails to load SHALL show that view's own error or notice state and SHALL NOT
reveal another view.

#### Scenario: Deep link reloads to its own view

- **WHEN** the operator pastes or reloads `http://<host>:<port>/workbench`
- **THEN** the shell loads and only the workbench view is visible

#### Scenario: Clicking a destination changes the URL and the view

- **WHEN** the operator clicks "Portfolio" in the sidebar
- **THEN** the address bar becomes the real path `/portfolio`, only the
  portfolio view is visible, and its sidebar link is marked active

#### Scenario: Back and forward switch views

- **WHEN** the operator navigates between two destinations and then presses the
  browser Back button, then Forward
- **THEN** each press restores the corresponding view, active nav state and
  document title without a full page reload

#### Scenario: Only one view is visible

- **WHEN** any single destination is active
- **THEN** the other destination sections are not simultaneously visible

### Requirement: Project fleet actions target real destinations

The fleet table SHALL send the operator to a real destination, never a `#`
fragment. A managed project's row action SHALL navigate to the real workbench
path `/workbench` and load that project. An unmanaged project's "Manage" link
SHALL point at the real management path `/management`. The workspace
"not yet onboarded" hint SHALL also point at `/management`.

#### Scenario: Managed project opens the real workbench path

- **WHEN** the operator activates the row action for a managed project
- **THEN** the URL becomes `/workbench` and that project's workbench detail is
  loaded

#### Scenario: Unmanaged project manages on the real path

- **WHEN** the operator activates the "Manage" action for an unmanaged,
  non-conflicting project
- **THEN** the URL becomes `/management` and no `#fragment` is used

