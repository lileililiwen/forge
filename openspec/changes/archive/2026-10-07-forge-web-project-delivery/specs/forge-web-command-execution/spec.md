# forge-web-command-execution (delta)

## MODIFIED Requirements

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
