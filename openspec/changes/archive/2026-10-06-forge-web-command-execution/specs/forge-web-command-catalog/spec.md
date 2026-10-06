## MODIFIED Requirements

### Requirement: Truthful availability and safe navigation

Forge SHALL enable only implemented typed web routes and SHALL explain provider,
project capability, disabled, not-yet-web and CLI-only states with a safe next
step. When a command's Forge Core operation already has an implemented,
session-gated admin route, Forge SHALL report that command as `web` pointing at
the real route rather than as `cli_only` or `not-yet-web`, so the catalog's
disposition agrees with what a signed-in operator can actually run in the
browser. `feature add` and `spec generate` SHALL be reported as `web` with their
admin routes; commands with no implemented route SHALL keep their honest
disposition.

#### Scenario: Command requires unavailable provider

- **WHEN** a command requires a disabled or unavailable provider
- **THEN** the catalog identifies the provider prerequisite and does not present the action as runnable

#### Scenario: Command is terminal-only

- **WHEN** a command requires an interactive terminal, serves a transport, or is a developer/build operation
- **THEN** the catalog links to its CLI help and explains why there is no browser execution control

#### Scenario: Core-backed authoring command has a web route

- **WHEN** `feature add` or `spec generate` is exposed through an implemented session-gated admin action route
- **THEN** the catalog reports it `web` with that route and omits the CLI-only next step

#### Scenario: Unknown search result

- **WHEN** a search has no matching commands
- **THEN** the UI presents an accessible empty result without calling an arbitrary shell command
