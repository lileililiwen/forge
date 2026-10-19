## Why

Forge has two real local runtime owners but no stable user-facing entry point
that connects them: Docker Compose already models a multi-service application,
and `gitrunbykaru` can launch a local project directory as a managed process
session. Today users must know which sibling command to invoke, while Forge's
generic `use` facade hides that product boundary and does not explain how to
launch a Compose stack.

## What Changes

- Add `forge run` as the local runtime entry point, with explicit `compose`
  and `local` modes.
- Route Compose mode to the installed Docker Compose CLI with the project root,
  selected Compose file, profiles, services and remaining arguments preserved.
- Route local mode to the installed `gitrunbykaru` CLI for one local project
  directory, preserving its managed-session lifecycle and explicit options.
- Provide predictable discovery, errors and exit-code propagation when paths,
  runtime executables or delegated commands fail.
- Document that Compose owns multi-service topology and GitRunByKaru owns a
  single local application process session; Forge does not duplicate either
  service schema or process/session manager.

## BFS Impact Map

| Surface | Impact |
|---|---|
| CLI contract | New nested `forge run compose` and `forge run local` commands, help and argument routing. |
| Runtime owners | Docker Compose remains the multi-service graph and lifecycle owner; GitRunByKaru remains the local single-process session owner. |
| Filesystem | Resolve project root and Compose file without changing project files; resolve relative Compose paths from the selected root and retain absolute paths when explicitly supplied. |
| Process boundary | Spawn argument vectors directly, inherit stdio, propagate child exit status, and fail closed when required executables are missing. |
| Web command catalog | Ensure the new nested commands are discoverable with an explicit CLI-oriented workflow description; do not introduce a second runtime API/UI control plane. |
| Persistence / registry | No new database records, service schema, generated files, or implicit project registration. |
| Compatibility | Existing `forge use gitrunbykaru` and direct Docker Compose workflows remain unchanged. |
| Verification | Stub-executable tests cover exact argv, cwd, failure and exit behavior; CLI help/catalog checks and full local Gate cover integration. |

## Capabilities

### New capabilities

- `local-runtime-entrypoints`: Route explicit Compose-stack and GitRunByKaru local runtime commands through Forge.

### Modified capabilities

- `forge-web-command-catalog`: Add truthful CLI-only discovery rows for both local runtime routes.
- `ci-cd-system-model-documentation`: Document runtime routing and distinguish local development from CI, release and delivery.

## Non-goals

- A Forge-owned service graph or new manifest/contract schema.
- Starting multiple unrelated local process sessions under a new Forge supervisor.
- Remote repositories, publishing, deployment, health promotion or hosted CI execution.
- Automatic Compose or GitRunByKaru installation, network package installation, or runtime execution during project discovery.
- A web process manager or durable session registry.
