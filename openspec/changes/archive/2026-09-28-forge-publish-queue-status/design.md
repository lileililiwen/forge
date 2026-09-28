# Design: observable sequential Forge publish queue

## Implementation boundary

Primary owner is `/home/paul/code/forge` Rust Core/CLI:

- `src/main.rs`: fleet command and `deploy status` command.
- `src/publish/fleet.rs`: deterministic queue projection.
- `src/publish/providers.rs`: provider event transport and validation.
- `src/registry/mod.rs`: queue/run journal query and state persistence.
- `tests/provider_contract.rs` and a new status contract suite: protocol and
  CLI evidence.

Provider implementation is `/home/paul/code/jenkins-local`:
`adapters/forge-publish-provider.py` emits progress events and keeps all
Docker operations on Mac. No script is installed on Mac and no project source
is retained there.

## Language and runtime

- Forge: Rust, Cargo, SQLite, `serde_json`, existing clap CLI.
- Jenkins provider: Python 3, JSON stdin/stdout, JSON Lines progress on stderr.
- Target runtime: Mac SSH target with Docker Desktop and the existing runtime
  data under `/Users/allen/jenkins/runtime`.
- Required checks: `cargo test --all-targets`, provider unittest discovery,
  `cargo build`, `git diff --check`, and strict OpenSpec validation.

## Ownership and shared code

Forge owns queue identity, ordering, journal state, and status projection.
Jenkins owns Mac execution and emits only the generic provider event contract.
OpenPanel remains a separate provider. The event schema is provider-neutral;
it is not extracted into a sibling library because the transport is already a
small versioned JSON contract and a shared library would couple release
lifecycles.

## Behavioral model

| State | Meaning | Allowed next states |
|---|---|---|
| `queued` | Project is in the deterministic fleet order but not started | `running`, `cancelled` |
| `running` | Provider has started work for this project | `succeeded`, `failed`, `timed_out` |
| `succeeded` | Provider terminal response is healthy/done | terminal |
| `failed` | Provider or verification failed | terminal |
| `timed_out` | Forge provider deadline elapsed | terminal |
| `cancelled` | Operator cancellation was explicitly accepted before start | terminal |

The queue has one `running` project maximum. Forge assigns a queue/run ID and
project operation ID. Duplicate terminal events are ignored; a project cannot
move from a terminal state back to running. A provider event is accepted only
when contract, run ID, project ID, and operation ID match. Event detail is
bounded and credential-redacted before display or persistence.

The existing operations journal remains the durable source. Status reads are
read-only and never append operations. A disconnected process is represented
as `timed_out` or `failed` after the bounded provider timeout; it is never
reported healthy from the absence of an event.

## Contract and compatibility

Progress event, sent as one JSON object per stderr line:

```json
{
  "contract": "forge-publish-provider/0.1.0",
  "event": "publish.progress",
  "provider": "jenkins",
  "project_id": "alethefy",
  "operation_id": "publish-alethefy-<sha12>",
  "queue_id": "fleet-<id>",
  "phase": "build-and-run",
  "status": "started",
  "detail": "stage 4/5"
}
```

`queue_id` is added as an optional request/event field for fleet calls; single
project calls remain compatible when it is absent. Terminal stdout remains the
existing `forge-publish-provider/0.1.0` response. Unknown event types are
ignored with a diagnostic; malformed progress events are recorded as provider
protocol failures only when they claim the active operation.

CLI surface:

```text
forge deploy status
forge deploy status --project alethefy
forge deploy status --queue <queue-id>
forge deploy status --watch --interval-secs 2
```

JSON output includes queue ID, eligible count, completed count, active project,
per-project state, provider, phase, timestamps, and bounded failure detail.
Human output includes one row per project and a final aggregate. `--watch`
polls at a bounded interval and exits when all projects are terminal or when
the command timeout is reached.

## Failure and boundary policy

- Empty eligible fleet: refuse before queue creation.
- Duplicate project IDs: refuse the registry as invalid.
- Provider disabled or missing: mark the project failed with the provider
  reason; with `--fail-fast`, stop before starting the next project.
- Provider timeout: persist `timed_out`, clean temporary Mac source through the
  provider finally path, and do not start the next project until the current
  process is reaped.
- Partial fleet: preserve successful projects and expose failed projects;
  aggregate command exits non-zero.
- Stale journal entry on a new Forge invocation: reconcile pending entries as
  interrupted failures using existing registry behavior.
- Secret-like event detail: redact or reject before persistence.
- Status with no history: return an explicit empty report and non-success exit
  only when a project/queue was requested and is unknown; fleet-wide empty
  history is a valid `empty` report.

## Verification oracle

- Unit test queue transitions and the one-running-project invariant.
- Provider contract fixture test verifies event ordering, malformed event
  handling, bounds, redaction, and terminal response compatibility.
- CLI contract tests verify fleet status, project filtering, JSON/human parity,
  `--watch` terminal exit, unknown project, and read-only journal behavior.
- Integration fixture runs two fake providers and proves project B is not
  invoked until project A has emitted a terminal response.
- Real Mac evidence, when available, must show provider progress events,
  healthy Compose output, route refresh, and a final Forge status row. Build
  success alone is insufficient.

## Decision ledger

- Resolved: queue execution is sequential, not parallel, to control Mac CPU,
  memory, Docker cache contention, and deployment ordering.
- Resolved: provider progress uses the existing child-process transport; no
  Mac callback server or permanent Mac script is introduced.
- Resolved: SQLite operations journal is the status source; no new database.
- Deferred: remote HTTP/WebSocket status API. The CLI and existing Forge API
  operation lookup are sufficient for this change.
- Deferred: automatic retry. A failed project is visible and can be manually
  republished; retries must not silently rebuild the whole fleet.
