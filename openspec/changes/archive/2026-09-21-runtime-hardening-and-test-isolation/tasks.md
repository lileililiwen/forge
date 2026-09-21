## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map registry path inputs and subprocess timeout owners across CLI, MCP, release, policy, docs, analytics and deployment callers.
- [x] 1.2 Add fixtures for read-only default-home execution, temporary registry ownership, timeout child cleanup and non-zero adapter output.
- [x] 1.3 Reconcile the proposal, design and capability specs; record direct-child versus descendant cleanup boundaries.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement isolated registry selection for all affected tests and prove the MCP round-trip does not touch host state.
- [x] 2.2 Implement kill-and-reap cleanup in the release adapter timeout path and preserve typed failure/journal semantics.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Exercise CLI, MCP and release retry behavior together with read-only homes, missing binaries, timeouts and repeated runs.
- [x] 3.2 Recheck credential-safe diagnostics, operation states, direct-child cleanup and unchanged success contracts; remove placeholders.

## 4. Verification

- [x] 4.1 Run the isolated test, full `cargo test`, `cargo clippy --all-targets -- -D warnings`, and the timeout fixture; record unavailable environment checks.
- [x] 4.2 Run `node scripts/check-openspec-change-names.mjs`, strict OpenSpec validation and `git diff --check`.
- [x] 4.3 Archive only after evidence satisfies every scenario; promote specs and update HANDOFF in the repository workflow.
