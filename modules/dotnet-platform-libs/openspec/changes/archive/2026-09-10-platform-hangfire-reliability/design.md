## Context

`Platform.Jobs.Hangfire` owns the adapter lifecycle while applications own storage selection and retry policy. The observed failure indicates an end-to-end test may dispose the Hangfire service provider or in-memory storage before enqueueing, or may share state across tests.

## Goals / Non-Goals

**Goals:**

- Isolate each end-to-end test's Hangfire storage and service provider.
- Prove enqueue, execution, context restoration, cancellation, and disposal behavior.
- Keep production behavior compatible with in-memory and PostgreSQL Hangfire storage.

**Non-Goals:**

- Replace Hangfire or introduce another scheduler.
- Change automatic retry semantics or own application job definitions.
- Mask the failure by weakening or skipping the regression test.

## Decisions

- Give each end-to-end test a fresh host/service provider and a uniquely owned in-memory storage instance.
- Start hosted services before enqueueing and await a bounded execution signal rather than using arbitrary sleeps.
- Dispose the host only after assertions complete; make test cleanup idempotent and cancellation-aware.
- Add a PostgreSQL test only as an opt-in Docker-gated integration test; deterministic lifecycle tests must run without Docker.

## Risks / Trade-offs

- [Risk] Timing-sensitive tests remain flaky → use task completion sources, bounded polling, and explicit server-start readiness.
- [Risk] A test-only workaround hides a production lifecycle issue → add direct tests around adapter ownership and document the disposal contract.
- [Risk] Hangfire version changes alter test APIs → isolate compatibility code in test helpers and retain analyzer warnings as follow-up work.

## Migration Plan

1. Reproduce the failure with the smallest test fixture.
2. Correct ownership/startup/cleanup ordering.
3. Add regression tests and run the full platform test suite.
4. Record any remaining pre-existing warnings in the handoff.