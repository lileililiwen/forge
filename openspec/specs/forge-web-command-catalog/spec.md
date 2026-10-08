# forge-web-command-catalog Specification

## Purpose
Expose an authenticated, versioned web catalog of every CLI command with truthful availability, structured execution blocks, and no generic shell execution.
## Requirements
### Requirement: Exhaustive CLI command coverage

Forge SHALL represent every top-level and nested CLI command in an authenticated, versioned web catalog with a stable ID, user-facing purpose, category, scope, risk, and web route or explicit CLI-only disposition.

#### Scenario: New CLI command added

- **WHEN** a top-level or nested Clap command is added without catalog metadata
- **THEN** the command coverage check fails and identifies the missing command path

#### Scenario: Catalog is complete

- **WHEN** an authenticated operator opens command search
- **THEN** every command path from the Rust Clap tree appears once with a valid category and disposition

### Requirement: Truthful availability and safe navigation

Forge SHALL enable only implemented typed web routes and SHALL explain
provider, project capability, disabled, not-yet-web and CLI-only states with a
safe next step. When a command's Forge Core operation already has an
implemented, session-gated admin route, Forge SHALL report that command as
`web` pointing at the real route rather than as `cli_only` or `not-yet-web`,
so the catalog's disposition agrees with what a signed-in operator can
actually run in the browser. `feature add`, `feature remove`,
`feature upgrade`, `spec generate`, `spec apply`, `deploy apply`,
`deploy plan`, `release apply`, `release plan` and `publish` SHALL be reported
as `web` with their admin routes; `delivery.status`, `delivery.preflight`,
`delivery.stage`, `delivery.promote` and `delivery.hermora-retry` SHALL be
reported as `web` with their session-gated project-delivery admin routes;
`new`, `import` and `register` SHALL be reported as
`web` with their session-gated project-management admin routes;
`check` SHALL be reported as `web` pointing at the read-only project status
route (`GET /v1/admin/projects/{id}/status`); `fleet status` SHALL be reported
as `web` pointing at the read-only fleet readiness summary route
(`GET /v1/admin/status`); commands with no implemented route SHALL keep their
honest disposition.

Every executable `web` row SHALL additionally carry a structured `execution`
block naming the exact admin `route`, HTTP `method`, the ordered typed
`parameters` (name, scalar kind, required flag), a `confirm_required` flag, a
`digest_bound` flag and the project-scope `risk`, so the browser can render a
runnable confirm-gated control directly from the catalog. An executable `web`
row whose command takes no browser-supplied input after server-side resolution
SHALL carry an empty typed-parameter list. A non-`web` row SHALL carry no
`execution` block. The `execution.route` of any row SHALL be one of the known
implemented web routes, so a self-described executable command can never point
at a route that does not exist.

#### Scenario: Command requires unavailable provider

- **WHEN** a command requires a disabled or unavailable provider
- **THEN** the catalog identifies the provider prerequisite and does not present the action as runnable

#### Scenario: Command is terminal-only

- **WHEN** a command requires an interactive terminal, serves a transport, or is a developer/build operation
- **THEN** the catalog links to its CLI help and explains why there is no browser execution control and carries no execution block

#### Scenario: Core-backed authoring command has a web route

- **WHEN** `feature add`, `feature remove`, `feature upgrade`, `spec generate` or `spec apply` is exposed through an implemented session-gated admin action route
- **THEN** the catalog reports it `web` with that route and omits the CLI-only next step

#### Scenario: Project creation command has a web route

- **WHEN** `new`, `import` or `register` is exposed through an implemented session-gated admin project-management route
- **THEN** the catalog reports it `web` with that route and a `local_write` `execution` block whose typed parameters are that command's structured fields

#### Scenario: Read-only status commands have a web route

- **WHEN** `check` and `fleet status` are exposed through the read-only project status and fleet readiness summary admin routes
- **THEN** the catalog reports each `web` with its route and no longer reports `check` as machine-stdout-only or `fleet status` as the fleet list, and neither carries an `execution` block

#### Scenario: Unknown search result

- **WHEN** a search has no matching commands
- **THEN** the UI presents an accessible empty result without calling an arbitrary shell command

#### Scenario: Executable web row carries an execution block

- **WHEN** a signed-in operator fetches the command catalog
- **THEN** every executable lifecycle row carries a structured `execution` block (route, method, typed parameters, `confirm_required`, `digest_bound`, risk) that names only a known implemented web route, and every non-`web` row carries no `execution` block

#### Scenario: Deploy command has an admin route

- **WHEN** `deploy plan` and `deploy apply` are exposed through the implemented session-gated admin deploy routes
- **THEN** the catalog reports `deploy plan` `web` with its read-only route and `deploy apply` `web` with a `remote_write` `execution` block, and neither keeps a CLI-only or project-capability next step

#### Scenario: Release command has an admin route

- **WHEN** `release plan` and `release apply` are exposed through the implemented session-gated admin release routes
- **THEN** the catalog reports `release plan` `web` with its read-only route and `release apply` `web` with a `remote_write` `execution` block carrying a required typed `version` parameter, and neither keeps a CLI-only, not-yet-web or provider-required next step

#### Scenario: Publish command has an admin route

- **WHEN** the bare `publish` command is exposed through the implemented session-gated admin publish route
- **THEN** the catalog reports it `web` with a `remote_write` `execution` block whose typed-parameter list is empty (the provider, project and revision are server-resolved), and it no longer keeps a provider-required or CLI-only next step

#### Scenario: Delivery commands have admin routes

- **WHEN** all staged project-delivery admin routes are implemented
- **THEN** the catalog reports status as a `web` read and the four mutations as `web` rows with valid executable blocks

#### Scenario: Command still has no route

- **WHEN** a delivery-related command has no implemented admin route
- **THEN** the catalog retains its honest non-web disposition and safe next step

### Requirement: Catalog authorization and invocation isolation

Forge SHALL require a valid global admin session to read the catalog and SHALL NOT provide a generic endpoint that executes arbitrary CLI text.

#### Scenario: Anonymous catalog request

- **WHEN** an unauthenticated client requests the command catalog
- **THEN** Forge returns 401 and no catalog data

#### Scenario: Search text contains shell syntax

- **WHEN** search text contains shell metacharacters
- **THEN** it is treated only as literal catalog text and is never executed

