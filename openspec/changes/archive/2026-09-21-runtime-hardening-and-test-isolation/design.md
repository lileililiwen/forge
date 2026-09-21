## Context

The repository has a broad contract suite, but at least one test reaches the
user-level default registry. The default location is an external mutable
resource and can be read-only in containers, sandboxes, or CI. The release
engine's `run_with_timeout` path can return on timeout while its child remains
alive.

## Goals / Non-Goals

**Goals:** isolate test state; make timeout termination and reaping consistent;
prove no stale child or partial journal claim remains after failure.

**Non-Goals:** changing timeout durations, adding a process supervisor, or
claiming that killing a direct child also kills arbitrary descendants on every
operating system.

## Decisions

Tests that exercise registry-backed dispatch pass an explicit temporary path or
set a scoped test-only environment through a serialized helper. Production code
continues to use the documented path precedence. Every timeout runner owns the
child until it has either returned output or performed kill-and-wait cleanup;
the release runner adopts the same behavior as policy, docs, analytics and
deployment. Timeout results retain the existing typed failure classification.

Where descendant cleanup is platform-dependent, the contract records direct
child termination as guaranteed and reports provider failure; stronger process
group cleanup is a separate future decision.

## Requirement and scenario coverage

- **R1 — Isolated registry-backed tests:** cover writable temporary state,
  read-only default home, and two sequential sessions.
- **R2 — Bounded release subprocesses:** cover success, timeout cleanup, and
  non-zero/invalid output without a false success journal.

## Failure and compatibility

The existing registry override remains authoritative. A test must fail with a
clear setup error if its explicit temporary path cannot be opened; it must not
fall back to the host registry. A timed-out release stage is partial/failed as
currently defined and is retryable only through the existing release state
machine.

## Migration Plan

No data migration. Existing release state is read using its current schema.
Only subprocess ownership and test setup change.

## Verification Strategy

Run the isolated failing MCP test under a read-only HOME, the full unit and
contract suite, clippy, and a fixture adapter that records whether it remains
alive after timeout. Run strict OpenSpec and diff checks before archive.

## Open Questions

The exact cross-platform process-group strategy remains open and must not be
silently introduced by this package.
