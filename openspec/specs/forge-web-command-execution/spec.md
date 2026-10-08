# forge-web-command-execution Specification

## Purpose
Execute handler-backed CLI commands from the browser under the same confirm-and-digest discipline the CLI uses, with no shell or argv interpolation.
## Requirements
### Requirement: Browser execution of handler-backed authoring commands

Forge SHALL expose, on the session-gated global admin surface, typed endpoints
that execute the project authoring commands already backed by in-process Core
handlers (`feature add` and `spec generate`) by delegating to the same handlers
the CLI and `/v1` bearer routes use. Each endpoint SHALL accept only the
structured fields of that command (a feature id and optional version; a non-empty
set of finding ids and an optional reason) and SHALL NOT interpret any request
field as a shell command, argv vector, filesystem path, or arbitrary CLI text.

#### Scenario: Signed-in operator previews and confirms an action

- **WHEN** an authenticated admin session submits the structured fields for one of these actions
- **THEN** Forge returns a bounded preview with a `plan_digest` and performs no write until the operator confirms with that exact digest

#### Scenario: Unsigned client attempts an action

- **WHEN** a request without a valid admin session cookie calls either action route
- **THEN** Forge refuses it the same way it refuses the other admin routes and runs no Core operation

#### Scenario: No generic text execution exists

- **WHEN** a caller attempts to pass CLI text, a shell string, or an argv vector to an action route
- **THEN** Forge treats it only as that command's structured field or rejects it, and never spawns a shell or interpolates argv

### Requirement: Confirm and digest binding on authoring web mutations

Forge SHALL require, for both `feature add` and `spec generate` on the admin
surface, `confirm` to be true and a `plan_digest` that matches the digest of the
exact project and field set the operator previewed, enforced in the admin layer
independently of the bearer route. A mutation that is not confirmed, or whose
digest does not match, SHALL be refused with a typed safe error and SHALL NOT
invoke the Core handler's write. Only on a matching digest SHALL Forge delegate to
the same Core handler the equivalent CLI command runs, producing an identical
result.

#### Scenario: Mutation without confirmation

- **WHEN** a signed-in operator submits one of these actions without `confirm` true
- **THEN** Forge returns the preview and digest, performs no write, and does not run the handler's mutation

#### Scenario: Mutation with a mismatched digest

- **WHEN** a signed-in operator submits one of these actions whose `plan_digest` does not match the reviewed project and field set
- **THEN** Forge refuses the request, returns a fresh digest, and invokes no Core write

#### Scenario: Confirmed mutation matches the CLI

- **WHEN** a signed-in operator submits one of these actions with `confirm` true and a matching `plan_digest`
- **THEN** Forge runs the same Core handler the equivalent CLI command runs and returns its typed result

### Requirement: Catalog agrees with what the browser can run

Forge SHALL report a command as `web` with its real admin route if and only if
that route is implemented and listed among the known web routes; a command with
no implemented route SHALL keep its `cli_only`, `not_yet_web`, disabled, project
capability, or provider disposition. `deploy apply` and `deploy plan` SHALL be
served by the admin deploy routes under the same confirm and digest discipline
as the authoring commands, delegating to the in-process deploy engine;
`release apply` and `release plan` SHALL be served by the admin release routes
under the same confirm and digest discipline, delegating to the in-process
release engine with the manifest's stages; `publish` SHALL be served by the admin
publish routes under the same confirm and digest discipline, resolving the
provider id, the provider configuration and the committed revision only
server-side; `delivery.status` SHALL be served by the admin delivery-status
route; `delivery.preflight`, `delivery.stage`, `delivery.promote` and
`delivery.hermora-retry` SHALL be served by their admin delivery routes under
the same confirm and digest discipline, resolving the provider and revision
only server-side; `new`, `import` and `register` SHALL be served by the
session-gated project-management routes under the same confirm and digest
discipline and resolve their destination only from the server-side configured
root; and `doctor`, `inspect` and `upgrade` SHALL remain served by the
existing workbench routes.

#### Scenario: Handler-backed command becomes runnable

- **WHEN** `feature add`, `spec generate`, the `deploy plan` / `deploy apply` commands, the `release plan` / `release apply` commands, the `publish` command, or the `new` / `import` / `register` creation commands gain their implemented admin route
- **THEN** the catalog reports that command `web` with that route and omits the CLI-only next step

#### Scenario: Handler-backed delivery command becomes runnable

- **WHEN** `delivery.status`, `delivery.preflight`, `delivery.stage`, `delivery.promote` or `delivery.hermora-retry` gains its implemented admin route
- **THEN** the catalog reports that command `web` with that route and omits any terminal-only next step

#### Scenario: Command still has no route

- **WHEN** a command's Core operation has no implemented admin route, such as a transport, build, PTY or interactive agent-process command
- **THEN** the catalog keeps its honest non-web disposition and a safe next step

