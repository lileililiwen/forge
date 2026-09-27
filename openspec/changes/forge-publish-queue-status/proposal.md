# Proposal: observable sequential Forge publish queue

## Why

Forge currently invokes external publish providers and receives a terminal
response, but an operator cannot query a durable deployment status while a
fleet is running. A fleet must be visibly sequential, and Jenkins/Mac progress
must be observable by Forge rather than hidden until the child process exits.

## What Changes

- Define a sequential fleet queue contract: one project is active at a time;
  the next project starts only after the previous project has a terminal state.
- Define secret-free provider progress events from `jenkins-local` to Forge.
- Persist project-level publish states and progress evidence in Forge's
  existing SQLite operations journal.
- Add `forge deploy status` query forms for fleet-wide and project-specific
  status, with JSON and human output; add a bounded `--watch` mode.
- Preserve provider independence: Jenkins and OpenPanel remain external
  providers and no provider code moves into Forge.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-publish-queue-status` | Operators can observe one sequential Forge publish queue and query its durable status | Forge / Rust plus Jenkins provider / Python | `forge-publish-provider/0.1.0`, `publish.progress`, SQLite `operations`, `forge deploy status` | Existing Forge provider orchestration and Jenkins Mac provider | Provider fixture emits ordered events; CLI status reports terminal and active states |

This is one package because queue ordering, progress ingestion, journal state,
and status querying form one operational capability. OpenPanel integration is a
consumer of the provider contract and is not changed here.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge registry | `src/registry/mod.rs`: `OperationEntry`, `record_operation`, `operations_for_project` | Existing SQLite operations journal and read models | Needs active progress representation and queue identity | Forge / Forge release | **Extend shared owner** |
| Forge provider boundary | `src/publish/providers.rs`: `invoke_provider`, provider contract | External provider process, timeout, response validation | Current implementation consumes only terminal stdout | Forge / provider contract release | **Extend shared owner** |
| Jenkins provider | `jenkins-local/adapters/forge-publish-provider.py`: `progress`, publish stages | Mac Docker build/run and runtime evidence | Needs stable event ordering and bounded event details | jenkins-local / Jenkins release | **Adapt through a generic adapter** |
| OpenPanel | Existing standalone provider contract work | Future provider can emit the same events | No change required for this package | OpenPanel release boundary | **Adapt through a generic adapter** |

No shared `common` project is introduced. Queue state belongs to Forge because
Forge owns orchestration and the operator-facing status command; providers
only emit provider-neutral events.

## BFS Impact Map

| Surface | Impact |
|---|---|
| Fleet CLI | Sequential queue remains deterministic; active project and queue position become visible. |
| Provider protocol | Add secret-free progress events on the provider transport without changing terminal response compatibility. |
| Persistence | Journal active/terminal project states and queue/run identity; reads do not create journal rows. |
| Status CLI | Add fleet and project status views with stable exit behavior and bounded watch polling. |
| Jenkins/Mac | Emit queued, preflight, transfer, build-and-run, verify, routing, and completed events; retain Mac-only build/runtime boundary. |
| Failure behavior | Provider timeout, malformed event, disconnected provider, and partial fleet completion remain explicit failures; no failure is reported as success. |
| Unaffected | GitHub webhook verification, OpenPanel implementation, Mac runtime data layout, and Docker cache policy are not redesigned. |

## Capabilities

1. `forge-sequential-publish-queue` — deterministic one-at-a-time fleet
   execution with per-project state.
2. `forge-publish-progress-events` — provider-to-Forge live progress contract.
3. `forge-deploy-status-query` — durable CLI status and bounded watch output.

## Non-goals

- Parallel fleet builds.
- Moving Jenkins or OpenPanel source into Forge.
- Building on Linux or GitHub Actions.
- Treating Docker logs as the durable Forge status source.
- Adding a second database or a Mac-side status script.
