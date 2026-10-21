## ADDED Requirements

### Requirement: One runner runs every native command

Forge SHALL run every native command — the project's declared build command,
its declared test command, and the test command the `forge test` operation
runs — through one shared runner that owns the argument array or the shell,
the working directory, the environment layering, the redaction and the
bounded capture. Forge SHALL NOT keep a second runner for the same native
command contract, and the `forge test` operation SHALL NOT run a command
through a runner of its own.

#### Scenario: A native command runs under one runner

- **WHEN** a build command, a test command or a readiness fixture runs
- **THEN** it runs through the shared runner, with the same redaction, the
  same bounded capture and the same wall-clock bound

#### Scenario: The test operation keeps its shell semantics

- **WHEN** a profile's declared test command uses shell syntax
- **THEN** it still runs through a shell, and the command is honoured as
  written

#### Scenario: The test operation reports redacted output

- **WHEN** a test command writes output that carries a credential-looking
  string
- **THEN** the recorded evidence carries the redacted text, not the raw
  string

### Requirement: A native command runs under a bound and is killed with its tree

Forge SHALL run every native command under one wall-clock bound, SHALL place
the command in its own process group, and SHALL signal that whole group
before reaping the child when the bound elapses, so that a command that
spawned descendants leaves nothing behind. The bound SHALL NOT be a manifest
field, so that a project cannot raise it.

#### Scenario: A command that overruns the bound is refused

- **WHEN** a native command is still running when the bound elapses
- **THEN** the process group is signalled, the child is reaped, and the
  outcome reports that the command timed out

#### Scenario: The descendants of a timed-out command are gone

- **WHEN** a native command spawned a descendant and overran the bound
- **THEN** the descendant is gone with it, and the host is left with no
  process the runner created

#### Scenario: A command that cannot start is reported as before

- **WHEN** a native command cannot be started at all
- **THEN** the outcome reports the start error, and no timeout is claimed

### Requirement: A timed-out native command is reported as a timeout

Forge SHALL distinguish a command that overran the bound from a command that
exited with a failure status, and SHALL name the timeout when the bound was
the cause. Forge SHALL NOT report a timeout as an ordinary non-zero exit.

#### Scenario: The outcome names the timeout

- **WHEN** a native command is killed because the bound elapsed
- **THEN** the outcome records the timeout, carries no exit code, and the
  caller names the bound and the command in the refusal

#### Scenario: An ordinary failure still reports its exit status

- **WHEN** a native command exits with a non-zero status before the bound
- **THEN** the outcome records that exit code, and the caller reports the
  failure the way it always did
