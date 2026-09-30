## MODIFIED Requirements

### Requirement: Bounded live preview

Forge SHALL run a preview for an authorized project revision through a
bounded, project-scoped process session whose lifecycle is journaled
through the existing `operations` table. The long-lived API process SHALL
own the live preview session for a project; the short-lived CLI SHALL
start the profile runner and report readiness through the same bounded
lifecycle without leaving a detached process behind. On explicit stop, on
startup timeout, and on session drop, Forge SHALL terminate the runner's
entire process tree so the reserved port is released and no descendant
process survives.

#### Scenario: Preview becomes ready

- **WHEN** the configured profile process starts and responds within
  the startup window over the API
- **THEN** Forge persists the `ready` preview state, returns a
  `forge-studio-preview/0.1.0` envelope with `state: ready`, the
  reserved port, and a same-origin preview URL bound to that project
  and revision, and keeps that session alive until an explicit stop

#### Scenario: CLI readiness probe

- **WHEN** the operator starts a preview over the short-lived CLI
- **THEN** Forge runs the same bounded start, returns the `ready`
  envelope captured at readiness, and tears the preview down so no
  detached process survives the CLI invocation

#### Scenario: Preview startup failure

- **WHEN** the process times out, exits, or its requested port is
  occupied
- **THEN** Forge reports `state: failed` with the typed
  `studio-start-timeout` or `studio-port-unavailable` reason, cleans
  up only its own process, and does not claim the preview is available

#### Scenario: Preview stop releases the process tree

- **WHEN** the operator stops a ready preview whose runner spawned
  descendant processes (for example a package manager that spawns its
  dev server)
- **THEN** Forge terminates the whole process tree, the reserved port
  can be bound again immediately, and no descendant of the runner
  remains

#### Scenario: Startup timeout releases the process tree

- **WHEN** the runner does not bind the reserved port within the startup
  window
- **THEN** Forge terminates the whole process tree before persisting
  `failed`, so no descendant of the runner remains

#### Scenario: Project boundary

- **WHEN** a preview file or process request resolves outside the
  registered project root
- **THEN** Forge refuses it before reading or mutating the external
  path with `studio-project-scope`

#### Scenario: Idempotent stop

- **WHEN** the operator stops an already-stopped preview
- **THEN** Forge returns the same envelope with `state: stopped` and
  records exactly one `studio.preview.stop` journal row per request

#### Scenario: Bounded log capture

- **WHEN** the preview process writes more than the bounded log cap
  to stdout or stderr
- **THEN** Forge truncates with a marker, redacts credential-shaped
  strings, and never echoes the cap marker without the truncation
  suffix
