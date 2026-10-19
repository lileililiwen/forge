## Context and current behavior

`forge use` currently delegates `gitrunbykaru` as an external CLI and may use
its cataloged `npx -y` fallback. GitRunByKaru's installed CLI accepts a local
directory target, `--port`, `--no-open`, `--keep`, `--docker`, and `--json`;
it owns detection, preparation, readiness, the process tree and cleanup.
Docker Compose already owns a multi-service project graph and has its own
service/profile selection, dependency ordering and teardown semantics.

Forge has deployment adapters that consume Compose files, but deployment is
not a local development runner. A new local service manifest would duplicate
the existing Compose graph and create a contract Forge would then have to
maintain. Runtime sessions are transient and owned by the invoked runtime, so
the Forge registry is not an appropriate session store.

## Decision

Add one explicit `forge run` command group:

- `forge run compose [--project-root DIR] [--file FILE] [--profile NAME ...] [--] [COMPOSE ARGS...]`
  resolves and validates the project root and Compose file, then starts
  `docker compose` with a direct argv vector. The default action is `up`; an
  explicit action such as `down`, `logs` or `ps` is forwarded. If the first
  passthrough token is an option, Forge inserts `up` before it. Repeated
  profiles and services are forwarded as Compose arguments rather than
  interpreted by Forge. A relative file is resolved from the project root.
- `forge run local [PATH] [--port PORT] [--keep] [--docker] [--json]`
  canonicalizes an existing directory and invokes an already installed
  `gitrunbykaru` executable with that directory and the selected options.
  This path does not use `npx`; a missing executable returns an install hint.
  This avoids turning a local run request into an implicit package download.

Both paths run with the selected project root as cwd, inherit stdin/stdout/
stderr, and propagate the exact representable child exit code. Spawn errors
and signal-only termination return a stable nonzero code and diagnostic.
Forge does not parse Compose YAML, modify project files, register the project,
write session state, or claim readiness itself.

The CLI parser must avoid eating Compose's flags. Parse Forge-owned flags only
before `--`; preserve every token after `--` verbatim. Provide short explicit
forms for common options without making an unbounded generic shell command.

## Ownership and failure behavior

| Boundary | Owner / behavior |
|---|---|
| Compose service graph, profiles, health and process lifecycle | Docker Compose; Forge delegates only. |
| Local app detection, install, launch, readiness and cleanup | GitRunByKaru; Forge delegates only. |
| Missing or invalid project path/file | Forge refuses before spawning. |
| Missing runtime executable | Forge fails with an install/configuration hint; no package-manager fallback. |
| Child nonzero exit | Preserve status and inherited diagnostic output. |
| Child terminated by signal / spawn failure | Stable nonzero status, diagnostic on stderr. |
| Persistent state | None in Forge. Runtime owners retain their own behavior. |

## Test strategy

Use temporary directories and executable stubs selected through a narrowed
`PATH`. The stubs record cwd and argv and return configured exit codes. Cover
Compose defaults (including options without an explicit action), explicit file/root, multiple profiles and passthrough args;
local default path and each supported option; paths containing spaces; missing
directories/files/binaries; spawn failure; and child exit propagation. Verify
that no command is launched on invalid inputs and that no files are created
outside the test temp directory. Add CLI help and command-catalog assertions.

## Rollout

The feature is additive and requires users to have Docker Compose or
GitRunByKaru installed. Existing direct runtime commands remain valid. The
release does not install or publish either external executable.
