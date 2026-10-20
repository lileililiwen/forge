# local-runtime-entrypoints Specification

## Purpose
Running a project locally needed Docker Compose or GitRunByKaru, and Forge had
no command that started either one. Two local runtime entry points delegate the
work: `forge run compose` hands the multi-service graph and its lifecycle to
Docker Compose, and `forge run local` hands one application directory to an
installed GitRunByKaru session. Forge resolves the inputs and spawns the child
without a shell, inheriting its streams and exit code, and never claims to
supervise services or persist a session.
## Requirements
### Requirement: Compose stack entry point

Forge SHALL provide `forge run compose` to delegate local multi-service
application lifecycle to Docker Compose. Forge SHALL resolve the project root
and selected Compose file before spawning, default the action to `up` when no
action token is supplied, preserve explicit Compose actions and arguments,
and SHALL NOT parse or rewrite the
Compose service graph. Forge SHALL inherit the caller's standard streams and
run the child with the selected project root as its working directory.

#### Scenario: Start a project Compose graph

- **WHEN** a user runs `forge run compose` from a project with one supported Compose file
- **THEN** Forge runs `docker compose -f <resolved-file> up` from the project root and returns the child result

#### Scenario: Select a file, profile, service and action

- **WHEN** a user supplies an explicit project root and Compose file, repeated profiles, a service selection and an explicit Compose action after `--`
- **THEN** Forge resolves only the root and file, forwards the Compose tokens in order, and leaves service/profile semantics to Docker Compose

#### Scenario: Compose options without an action still start the stack

- **WHEN** a user supplies Compose options after `--` but no action token
- **THEN** Forge inserts `up` before those options and delegates the resulting argv to Docker Compose

#### Scenario: Missing Compose input refuses before execution

- **WHEN** the root is not a directory, the selected Compose file is missing, or no default Compose file can be resolved
- **THEN** Forge reports the invalid input, exits nonzero and does not spawn Docker Compose

#### Scenario: Compose executable is unavailable

- **WHEN** a valid Compose invocation is requested but Docker is not installed or the Docker executable cannot be spawned
- **THEN** Forge reports an actionable installation hint, exits nonzero and does not attempt another package manager or network installation

### Requirement: GitRunByKaru local process entry point

Forge SHALL provide `forge run local` to delegate one local application
directory to an already installed `gitrunbykaru` executable. Forge SHALL
canonicalize and validate the directory, pass explicitly selected supported
options without rewriting the project, inherit standard streams and run the
child from that directory. Forge SHALL NOT use `npx` or another package manager
to install or resolve the runtime implicitly. Detection, preparation,
readiness and process/session cleanup remain owned by GitRunByKaru.

#### Scenario: Launch the current local project

- **WHEN** a user runs `forge run local` in a valid application directory and GitRunByKaru is installed on PATH
- **THEN** Forge invokes GitRunByKaru with the canonical local directory and returns its process result

#### Scenario: Forward selected runtime options

- **WHEN** a user selects a preferred port, Docker isolation, keep behavior or JSON output
- **THEN** Forge forwards the corresponding GitRunByKaru options unchanged in meaning and does not interpret its session output

#### Scenario: GitRunByKaru is missing

- **WHEN** a local run is requested and GitRunByKaru is not installed on PATH
- **THEN** Forge exits nonzero with an installation hint and SHALL NOT invoke `npx`, npm or a network operation

#### Scenario: Invalid local directory refuses before execution

- **WHEN** the selected path does not exist or is not a directory
- **THEN** Forge reports the path error, exits nonzero and does not spawn GitRunByKaru

### Requirement: Runtime delegation result semantics

Both local runtime entry points SHALL spawn argument vectors without a shell,
inherit stdin/stdout/stderr, preserve the child's exit code when available,
and return a stable nonzero exit code with a diagnostic when spawning fails or
the child terminates without a numeric exit code. Forge SHALL NOT record a
passing readiness or session state from process creation alone.

#### Scenario: Child returns nonzero

- **WHEN** Docker Compose or GitRunByKaru exits with a nonzero code
- **THEN** Forge returns that code and leaves the child's output unchanged

#### Scenario: Child termination has no exit code

- **WHEN** the delegated process terminates by signal and the platform provides no numeric exit status
- **THEN** Forge emits a diagnostic and exits with the documented stable nonzero fallback

#### Scenario: Child cannot be spawned

- **WHEN** process creation fails after input validation
- **THEN** Forge reports the executable and operating-system error and exits with a stable nonzero code

### Requirement: Local runtime command discovery

Forge SHALL include both local runtime entry points in CLI help and the
command-catalog surface. Descriptions SHALL identify Compose as the owner of
multi-service lifecycle and GitRunByKaru as the owner of one local app
session. The catalog SHALL NOT claim Forge itself supervises services or
persists runtime sessions.

#### Scenario: Discover runtime entry points

- **WHEN** a user requests CLI help or the Forge command catalog
- **THEN** `run compose` and `run local` appear with their required runtime prerequisites and delegation boundaries
