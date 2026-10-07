# forge-web-command-catalog (delta)

## MODIFIED Requirements

### Requirement: Truthful availability and safe navigation

Forge SHALL enable only implemented typed web routes and SHALL explain provider,
project capability, disabled, not-yet-web and CLI-only states with a safe next
step. When a command's Forge Core operation already has an implemented,
session-gated admin route, Forge SHALL report that command as `web` pointing at
the real route rather than as `cli_only` or `not-yet-web`, so the catalog's
disposition agrees with what a signed-in operator can actually run in the
browser. `feature add`, `feature remove`, `feature upgrade`, `spec generate`,
`spec apply` and `deploy apply` SHALL be reported as `web` with their admin
routes, and `deploy plan` SHALL be reported as `web` with its read-only admin
route; commands with no implemented route SHALL keep their honest disposition.

Every executable `web` row SHALL additionally carry a structured `execution`
block naming the exact admin `route`, HTTP `method`, the ordered typed
`parameters` (name, scalar kind, required flag), a `confirm_required` flag, a
`digest_bound` flag and the project-scope `risk`, so the browser can render a
runnable confirm-gated control directly from the catalog. A non-`web` row SHALL
carry no `execution` block. The `execution.route` of any row SHALL be one of the
known implemented web routes, so a self-described executable command can never
point at a route that does not exist.

#### Scenario: Command requires unavailable provider

- **WHEN** a command requires a disabled or unavailable provider
- **THEN** the catalog identifies the provider prerequisite and does not present the action as runnable

#### Scenario: Command is terminal-only

- **WHEN** a command requires an interactive terminal, serves a transport, or is a developer/build operation
- **THEN** the catalog links to its CLI help and explains why there is no browser execution control and carries no execution block

#### Scenario: Core-backed authoring command has a web route

- **WHEN** `feature add`, `feature remove`, `feature upgrade`, `spec generate` or `spec apply` is exposed through an implemented session-gated admin action route
- **THEN** the catalog reports it `web` with that route and omits the CLI-only next step

#### Scenario: Unknown search result

- **WHEN** a search has no matching commands
- **THEN** the UI presents an accessible empty result without calling an arbitrary shell command

#### Scenario: Executable web row carries an execution block

- **WHEN** a signed-in operator fetches the command catalog
- **THEN** every executable lifecycle row carries a structured `execution` block (route, method, typed parameters, `confirm_required`, `digest_bound`, risk) that names only a known implemented web route, and every non-`web` row carries no `execution` block

#### Scenario: Deploy command has an admin route

- **WHEN** `deploy plan` and `deploy apply` are exposed through the implemented session-gated admin deploy routes
- **THEN** the catalog reports `deploy plan` `web` with its read-only route and `deploy apply` `web` with a `remote_write` `execution` block, and neither keeps a CLI-only or project-capability next step
