# Proposal: revision-identifiable publish status

## Why

The current publish record is too coarse: `done` or `failed` does not show
whether image build or container startup failed. Operators also cannot identify
which Git revision a running container represents from Docker alone.

## What Changes

- Split every publish into independently observable `build` and `run` phases.
- Persist phase status, timestamps, evidence, and bounded failure detail in
  Forge status output.
- Include the committed Git SHA (at least the first 12 hexadecimal characters)
  in every container/project runtime name created by the provider.
- Make `forge deploy status` expose the revision and phase states needed to
  verify the currently running deployment.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-publish-observability-revision-containers` | A published revision is phase-visible and identifiable in Docker | Forge / Rust plus Jenkins provider / Python | Provider request/response, progress events, Compose project/container naming | `forge-publish-queue-status`, existing Mac provider | Status fixture and Docker inspect prove build/run and SHA identity |

These requirements share one revision identity and one publish lifecycle. They
are kept together because status phase identity and container identity must
refer to the same exact commit.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge provider contract | `src/publish/providers.rs` | Revision already travels in `PublishProviderRequest`; progress transport exists | Phase schema and runtime identity are not yet durable | Forge | **Extend shared owner** |
| Jenkins Mac provider | `jenkins-local/adapters/forge-publish-provider.py` | Mac Compose build/run and runtime overlays | Compose project name currently omits revision | Jenkins-local | **Adapt through a generic adapter** |
| Docker/Compose runtime | Mac Docker Desktop Compose project labels | Native project/container labels and image metadata | Explicit `container_name` projects may bypass generated names | Mac runtime data only | **Adapt through a generic adapter** |

No new shared library or Mac-side script is introduced.

## BFS Impact Map

| Surface | Impact |
|---|---|
| Provider request | Full Git SHA is mandatory and becomes the runtime identity input. |
| Progress protocol | Adds `phase=build|run`, terminal phase status, and revision metadata. |
| Forge journal/status | Stores build and run outcomes separately and renders failure evidence. |
| Compose invocation | Uses a revision-qualified project name; stale revisions are not treated as current. |
| Runtime verification | Checks container names/labels and health against the requested revision. |
| Compatibility | Existing provider terminal responses remain valid; old containers remain observable but are not current for a new revision. |
| Unaffected | GitHub webhook verification, PostgreSQL data layout, BuildKit cache policy, and OpenPanel internals. |

## Capabilities

1. `forge-publish-phase-status` — separate build and run status.
2. `forge-revision-runtime-identity` — commit hash in runtime container identity.

## Non-goals

- Rebuilding images on Linux or GitHub Actions.
- Deleting old revision containers automatically in this change.
- Moving source code or scripts onto the Mac.
- Replacing Docker/Compose with another runtime.
