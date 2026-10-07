# forge-web-command-catalog delta

## MODIFIED Requirements

### Requirement: Truthful availability and safe navigation

Forge SHALL enable only implemented typed web routes and SHALL explain provider,
project capability, disabled, not-yet-web and CLI-only states with a safe next
step. When a command's Forge Core operation already has an implemented,
session-gated admin route, Forge SHALL report that command as `web` pointing at
the real route rather than as `cli_only` or `not-yet-web`, so the catalog's
disposition agrees with what a signed-in operator can actually run in the
browser. `feature add`, `feature remove`, `feature upgrade`, `spec generate` and
`spec apply` SHALL be reported as `web` with their admin routes; commands with no
implemented route SHALL keep their honest disposition.

Every `web` row SHALL additionally carry a structured `execution` block naming
the exact admin `route`, HTTP `method`, the ordered typed `parameters`
(name, scalar kind, required flag), a `confirm_required` flag, a `digest_bound`
flag and the project-scope `risk`, so the browser can render a runnable
confirm-gated control directly from the catalog. A non-`web` row SHALL carry no
`execution` block. The `execution.route` of any row SHALL be one of the known
implemented web routes, so a self-described executable command can never point at
a route that does not exist.

#### Scenario: Command requires unavailable provider

- **WHEN** a command requires a disabled or unavailable provider
- **THEN** the catalog identifies the provider prerequisite and does not present the action as runnable

#### Scenario: Command is terminal-only

- **WHEN** a command requires an interactive terminal, serves a transport, or is a developer/build operation
- **THEN** the catalog links to its CLI help and explains why there is no browser execution control and carries no execution block

#### Scenario: Core-backed lifecycle command has a web route

- **WHEN** `feature add`, `feature remove`, `feature upgrade`, `spec generate` or `spec apply` is exposed through an implemented session-gated admin action route
- **THEN** the catalog reports it `web` with that route and an execution block describing its typed parameters, and omits the CLI-only next step

#### Scenario: Executable row describes how to run it

- **WHEN** the browser reads a `web` row
- **THEN** the row supplies a route, method, typed parameter list and confirmation requirement sufficient to render an action control without any bespoke per-command wiring

#### Scenario: Non-executable row carries no execution block

- **WHEN** the browser reads a row whose disposition is `cli_only`, `not-yet-web`, `provider_required`, `project_capability_required` or `disabled`
- **THEN** that row carries no `execution` block and is never rendered as a runnable control

#### Scenario: Unknown search result

- **WHEN** a search has no matching commands
- **THEN** the UI presents an accessible empty result without calling an arbitrary shell command
