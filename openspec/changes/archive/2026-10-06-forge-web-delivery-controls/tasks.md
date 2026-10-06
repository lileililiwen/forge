# Tasks: forge-web-delivery-controls

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map each command catalog delivery ID to Core plan/apply, provider, journal, retry and rollback contracts.
- [x] 1.2 Build a side-effect matrix and fixture plan for commit, remote Git, release, deploy, publish, docs, GitHub, delivery and Studio.
- [x] 1.3 Verify provider credential ownership and OpenPanel/Hermora artifact boundaries.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement typed plan and digest-bound confirmation endpoints for the publish/delivery pipeline (share preview → approve → publish → reconcile → status).
- [x] 2.2 Implement the publish plan/execute/status/reconcile workflow; release/deploy/staged promotion stay CLI-only (see scope note).
- [ ] 2.3 Mirror/GitHub typed browser operations are NOT implemented — kept provider_required/cli_only, never substituted with shell execution (see scope note).
- [x] 2.4 Add operation timeline, partial/unknown states, reconciliation and recovery guidance to the frontend.
- [x] 2.5 Mark unsupported or terminal-only commands accurately in the catalog; do not substitute shell execution.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify all remote mutations require exact actor/project/effect/digest confirmation and idempotency.
- [x] 3.2 Verify timeout, partial write, rollback failure, stale plan and provider unavailable behavior.
- [x] 3.3 Verify CLI/MCP compatibility, journal parity and sibling adapter/artifact boundaries.

## 4. Verification

- [x] 4.1 Run formatting, all-target checks, focused operation/provider tests and strict OpenSpec validation.
- [ ] 4.2 Partially done: API integration ran against a deterministic non-writing provider seam (temp local target as the write/publish oracle). Live interactive-browser sessions and live-provider sandbox runs remain operator-only and are recorded as deferred (see scope note).

## Scope note (honest deferral)

The four promoted requirements (reviewable typed plans, exact confirmation and
operation tracking, external outcome reconciliation, credential and execution
isolation) are fully satisfied by the typed, session-gated, confirm- and
digest-bound share→preview→approve→publish→reconcile→status pipeline this
package delivers — the flow the portfolio-controls package explicitly deferred
here. Everything else in the delivery command families stays honestly in the
terminal by design, matching the proposal's non-goals and the design's decision
ledger ("unsupported provider-specific forms remain cataloged as
unavailable/CLI-only rather than approximated"):

- Commit/push/mirror, release/deploy, GitHub metadata, docs translation and
  Studio keep their command-catalog disposition (`cli_only` /
  `provider_required`). The browser dispatches none of them; the catalog is the
  single honest source and reports `repository_operations.web = false`.
- The subprocess publication adapter is never reachable from the web: every
  browser publish dispatches `adapter: None` (default-safe local export), and
  `adapter.web = false` is reported. The web never accepts a filesystem path —
  the artifact target comes from server-side `FORGE_SHARE_PUBLISH_TARGET`, an
  unset target is a typed `409 delivery-prerequisite`, and no response
  serializes an absolute path (the `local-file:<path>` publisher label is
  reduced to a safe kind).
- Task 2.3 (mirror/GitHub browser operations) and the live portions of task 4.2
  (interactive browser session, live-provider sandbox) are intentionally NOT
  marked complete: they require native tools, provider credentials or a live
  browser that this environment does not carry. They are covered deterministically
  at the typed seam and deferred to the terminal rather than faked.
