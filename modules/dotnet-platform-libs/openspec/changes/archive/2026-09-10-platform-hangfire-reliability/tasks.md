## 1. Reproduce and isolate

- [x] 1.1 Reproduce the disposed `Hangfire.InMemory.State.Dispatcher` failure using the named regression test.
- [x] 1.2 Trace host, service-provider, storage, hosted-server, and test-fixture disposal order.
- [x] 1.3 Document the intended ownership boundary for production and test storage.

## 2. Reliability implementation

- [x] 2.1 Refactor the test host builder to create isolated storage and service-provider lifetimes.
- [x] 2.2 Add explicit worker readiness and job completion synchronization with bounded cancellation.
- [x] 2.3 Review adapter disposal and cancellation paths for double-disposal and post-disposal use.

## 3. Regression coverage

- [x] 3.1 Add sequential repeated-dispatch coverage.
- [x] 3.2 Add context-free, tenant-context, cancellation, and failed-job lifecycle coverage.
- [x] 3.3 Add parallel end-to-end test collection for cross-test isolation.
- [x] 3.4 Add post-disposal and worker-readiness-timeout regression coverage.
- [x] 3.5 Preserve and run the Docker-gated PostgreSQL storage test when Docker is available.

## 4. Verification

- [x] 4.1 Run the Hangfire test project repeatedly and serially.
- [x] 4.2 Run the full platform test suite and record warnings separately from failures.
- [x] 4.3 Run strict OpenSpec validation and `git diff --check`.
