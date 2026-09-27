# Design: forge-publish-plugin-orchestration

## Implementation boundary

Forge owns the publish command, event intake, provider registry, run state, and
provider protocol. The primary Rust surfaces are `src/main.rs`, a new
`src/publish/providers.rs`, and the existing registry/release/deploy modules.
Provider processes are external JSON-RPC-compatible executables selected by a
provider manifest. No OpenPanel or Jenkins source, crate, script, or runtime
is embedded in Forge core.

## Provider contract

Provider input is one JSON request per invocation:

```json
{
  "contract": "forge-publish-provider/0.1.0",
  "operation": "preflight|publish|verify|rollback",
  "provider": "openpanel",
  "project_id": "alethefy",
  "source": {"repository": "https://github.com/org/repo.git", "revision": "<sha>"},
  "folder": "/absolute/path/when-manual",
  "release": {"id": "...", "digest": "..."},
  "dry_run": false,
  "operation_id": "..."
}
```

The response is one JSON object containing the same contract, provider,
operation id, terminal status, redacted evidence, health state, and recovery
hints. Secret values, source contents, and arbitrary shell strings are
forbidden. Provider commands use argument arrays or provider-native typed APIs.

## Lifecycle and state

Provider states are `installed`, `enabled`, `disabled`, `unavailable`, and
`failed`. Forge refuses publish when the selected provider is disabled or
unavailable. Disabling a provider does not delete its history and takes effect
before the next invocation. An in-flight run finishes under its captured
provider identity; subsequent runs cannot start through that provider.

Publish states are `accepted`, `preflighted`, `publishing`, `healthy`, `failed`,
and `rolled_back`. The operation id is derived from project, provider,
revision, and trigger id so a webhook retry is idempotent.

## Trigger convergence

Manual CLI input and GitHub push events create the same `PublishRequest`.
Push intake must verify the GitHub signature, repository, branch/ref policy,
and full commit SHA before enqueueing. The folder form resolves a local
project manifest and captures its current revision. Neither trigger directly
calls a provider.

## OpenPanel and Jenkins ownership

OpenPanel’s `DeploymentAdapter`, `ApplicationDeliveryService`, and
`git_deployment` logic remain OpenPanel-owned in the standalone OpenPanel
repository. A thin provider facade translates Forge JSON requests into those
typed calls and translates evidence back. The Jenkins provider wraps existing
compatibility behavior in the standalone jenkins-local repository; it is not
the default and is not installed on the Mac as a script bundle.

## Failure boundaries and verification

- Invalid signature, unknown project, disabled provider, or contract mismatch:
  refuse before provider invocation.
- Provider timeout or malformed response: persist `failed` with redacted
  evidence; do not infer health.
- Post-publish health failure: request provider rollback when declared and
  preserve the failed release evidence.
- Duplicate push event: return the existing terminal result for the same
  operation id.
- Provider disabled during a run: do not interrupt the captured run; reject
  only new runs.

Verification requires Forge unit tests for lifecycle/idempotency, CLI tests for
project/folder forms, signed push-event tests, provider conformance fixtures,
and OpenPanel/Jenkins enable-disable integration tests.
