# forge-web-module-workflow Specification

## Purpose

Expose Forge's declared in-tree module inventory and existing Gate verification
workflow through its authenticated web portal.

## Requirements

## ADDED Requirements

### Requirement: Discover only Forge-owned declared modules

Forge SHALL expose a shared module catalog used by CLI and API transports. The
catalog SHALL include only valid `.project.json` declarations that are
immediate children of the running Forge source checkout's `modules/` directory
and have a supported in-tree Gate declaration. It SHALL reject invalid or
duplicate identities, malformed declarations, and paths that escape the
canonical module directory. The web API SHALL return relative paths only and
SHALL NOT accept a browser-supplied source root.

#### Scenario: A valid module catalog is available

- **WHEN** an authenticated administrator requests the module catalog
- **THEN** Forge returns modules sorted by ID with identity, stack, profile,
  relative path and latest Gate status, without absolute filesystem paths

#### Scenario: No modules are declared

- **WHEN** the Forge source checkout contains an empty `modules/` directory
- **THEN** the API returns an empty module list and the UI presents an empty
  state rather than an error or fabricated module

#### Scenario: A module declaration escapes its owner

- **WHEN** a module, project declaration or Gate declaration resolves outside
  its canonical module directory
- **THEN** discovery fails closed and returns no partial module inventory

#### Scenario: The running binary has no source module tree

- **WHEN** the API cannot resolve the Forge-owned source checkout or its
  `modules/` directory
- **THEN** the API reports an unavailable module catalog and does not search
  sibling projects or browser-supplied paths

### Requirement: Protect module APIs with Forge administrator authorization

The module inventory, Gate status and Gate run endpoints SHALL use the existing
Forge admin-session guard. Missing or invalid sessions SHALL receive the
existing authorization failure and SHALL receive no module metadata or Gate
evidence.

#### Scenario: An unauthenticated browser requests modules

- **WHEN** the module page requests inventory or status without a valid admin
  session
- **THEN** the API refuses the request and the UI presents a sign-in or
  session-expired state without exposing cached private module data

### Requirement: Report actual module Gate evidence

Forge SHALL provide authenticated module Gate status and run routes that
resolve one exact discovered module ID server-side and delegate to the existing
Gate engine and evidence functions. Real evidence SHALL be saved atomically
under the module and journaled through the Forge Registry. PASS, FAIL and
REVIEW_REQUIRED SHALL reflect the runtime result; missing or invalid runtime
output SHALL never become PASS.

#### Scenario: A module has no prior Gate evidence

- **WHEN** an administrator opens status for a valid module with no saved Gate
  evidence
- **THEN** Forge reports `unverified` and the UI does not label the module
  healthy or passed

#### Scenario: A module Gate run completes

- **WHEN** an administrator runs a valid module with `fast` or `full`
- **THEN** Forge returns the parsed Gate evidence, persists it under that
  module, journals the result, and the UI renders the actual aggregate,
  freshness, revision and check rows

#### Scenario: Gate reports a blocked verdict

- **WHEN** the Gate runtime returns FAIL or REVIEW_REQUIRED evidence
- **THEN** the API returns that evidence as a completed Gate result and the UI
  shows the blocked checks and recovery guidance instead of a transport error

#### Scenario: A Gate run cannot produce evidence

- **WHEN** the runtime is unavailable, malformed, times out, or evidence cannot
  be persisted
- **THEN** Forge returns an unavailable or typed error, preserves prior
  evidence, and the UI retains the last known status with a retry action

### Requirement: Constrain module Gate run inputs and concurrency

The module Gate run endpoint SHALL accept only `fast` or `full`, defaulting to
`full`, and SHALL reject arbitrary paths, commands, Gate configuration and
timeout values. It SHALL prevent concurrent Gate runs for the same module
within one API process while allowing different modules to run independently.

#### Scenario: A Gate request contains an invalid profile or extra field

- **WHEN** a Gate run request has a profile other than `fast` or `full` or
  includes fields outside the supported request contract
- **THEN** Forge rejects it before resolving or running a module

#### Scenario: A module already has an active run

- **WHEN** another Gate run is requested for the same module before the first
  run finishes
- **THEN** Forge returns 409 `module-gate-in-progress` without starting a
  second run; runs for different modules remain independent

### Requirement: Provide an accessible module workflow in the admin portal

The standalone web UI SHALL provide a Modules navigation destination that
lists valid modules, allows selection of a module and profile, and displays its
Gate status and check details. It SHALL provide truthful loading, empty,
unavailable, unauthorized, running, completed and failure states. The
interface SHALL support keyboard operation, visible focus, labelled controls,
announced run status, responsive reflow at narrow widths and no color-only
status communication.

#### Scenario: An administrator selects and verifies a module

- **WHEN** the administrator selects a module and activates Run Gate
- **THEN** the UI prevents duplicate activation for that module and updates the
  selected detail view from the API response

#### Scenario: The module list is empty or unavailable

- **WHEN** the catalog returns empty or unavailable
- **THEN** the UI distinguishes the empty state from unavailable source and
  provides the appropriate recovery guidance

#### Scenario: The operator uses keyboard or a narrow display

- **WHEN** the administrator navigates the module list and Gate controls by
  keyboard or uses the page at a 360px viewport
- **THEN** controls remain operable and labelled, focus remains visible, run
  outcomes are announced, and content reflows without horizontal page
  overflow
