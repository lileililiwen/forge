## Why

The current platform test run exposed a real reliability gap: `Platform.Jobs.Hangfire.Tests.EndToEndTests.Dispatched_jobs_run_context_free_without_an_ambient_context` failed because the Hangfire in-memory dispatcher was already disposed. Job infrastructure must be verified for lifecycle isolation before it is adopted by many applications.

## What Changes

- Make Hangfire test-host lifetime and storage ownership deterministic.
- Verify dispatcher, hosted server, scope, and storage disposal ordering.
- Add regression coverage for sequential and parallel end-to-end job execution.
- Preserve application-owned retry policy, queues, payloads, and persistent storage.

## Capabilities

### New Capabilities

- `platform-hangfire-reliability`: deterministic lifecycle and end-to-end reliability guarantees for the Hangfire adapter.

### Modified Capabilities

- `platform-jobs-hangfire`: strengthen lifecycle and end-to-end requirements.

## Impact

Affected code and tests are under `src/Platform.Jobs.Hangfire` and `tests/Platform.Jobs.Hangfire.Tests`. No public job contract or application persistence schema is intended to change.