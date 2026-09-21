# Proposal: Runtime hardening and test isolation

## Why

The current suite fails when the default Forge registry resolves to a read-only
workspace: the MCP round-trip test uses the process default instead of an
isolated registry and fails with `registry-error: attempt to write a readonly
database`. The release adapter timeout path also returns without killing and
reaping its child, unlike the other adapter runners. These are concrete gaps in
the roadmap's failure-boundary and repeatability requirements.

## What Changes

- Make transport and integration tests explicitly own their temporary registry
  and filesystem state.
- Make every bounded external-process timeout terminate and reap the child, and
  preserve a typed unavailable/timeout outcome.
- Add regression coverage for read-only default environments, timeout cleanup,
  and repeated invocation.

## BFS Impact Map

- **Capabilities and flows:** MCP stdio dispatch; release package/container/
  notes adapters; test and retry flows.
- **Modules, contracts and persistence:** `src/mcp`, `src/release/engine`,
  registry path selection, subprocess lifecycle and operation outcomes.
- **Callers:** CLI, MCP, release engine and contract tests; no new transport.
- **Dependencies and configuration:** temporary registry paths in tests;
  existing adapter environment variables and timeout settings remain valid.
- **Failure and boundary behavior:** read-only default homes, missing adapters,
  timeout, non-zero exit, descendant processes, and repeated retry.
- **Tests:** unit, transport contract, release adapter and cross-surface tests.
- **Compatibility/security/privacy:** no change to public success contracts;
  do not leak command arguments or credentials in timeout diagnostics.
- **Unaffected:** profile catalog, generated project templates, external
  provider availability, portal UI and release publication policy.

## Capabilities

### New Capabilities

- `runtime-hardening-and-test-isolation`: deterministic tests and bounded
  subprocess lifecycle safety.

### Modified Capabilities

- `release-publishing`: timeout failure must not leave a running adapter.
- `mature-mcp-surface`: round-trip tests must not depend on host registry state.

## Non-goals

This package does not add providers, change release stage semantics, redesign
the registry schema, or claim real integration evidence. It is not a CI or
packaging change.
