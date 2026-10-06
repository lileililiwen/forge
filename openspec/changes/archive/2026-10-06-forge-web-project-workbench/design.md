# Design: forge-web-project-workbench

## Implementation boundary

Forge Rust 2021 Core/API and standalone `frontend/`; inspect command handlers/modules corresponding to profile, kit, project lifecycle, doctor, feature, upgrade, spec, agent, component, UI-pattern, intent, procedure, standard, remediate, describe and classify. Do not change independent CLI semantics or put markup in Rust/API routes.

## Language and runtime

Rust 2021 MSRV 1.87, existing SQLite/API transport, browser-native HTML/CSS/JS. No Python or separate browser server runtime. Existing build/test commands plus module-focused tests and the API+Rust-web browser smoke are required.

## Ownership and shared code

Core modules own validation, deterministic plans, filesystem writes and journals. API handlers are typed adapters only. Frontend renders structured state. Do not invoke the Forge executable from the API or duplicate Core rules in JavaScript.

## Behavioral model

Every workflow resolves a managed project from an authenticated fleet identity, checks capability and source freshness, loads typed current state, then either returns a read-only inspection/plan or accepts a typed confirmed operation. Plan/diff operations have no side effects. Writes bind confirmation to the exact project, operation type and plan digest; accepted operations return a journal ID and report per-stage status. TTY-only operations remain CLI-only. Commands in scope: `list`, `inspect`, `register`, `import`, `new`, `graduation preview/import`, `profile list/inspect/resolve/preflight`, `kit pack/verify`, `doctor`, `check` (read-only report), `gate`, project `identity` session/config/challenge operations only where a typed non-TTY Core contract exists, `feature list/inspect/resolve/add/remove/upgrade`, `upgrade`, `test`, `spec generate/list/inspect/route/apply`, `agent start/pause/takeover/...` only where manager supports a non-TTY typed operation, `component`, `ui-pattern`, `intent`, `procedure`, `standard`, `remediate`, `describe`, and `classify`.

## Contract and compatibility

Add typed authenticated JSON resources per workflow family. Stable request shapes reuse Core types; mutation requests include idempotency key and plan digest where applicable. Never accept filesystem paths outside a project root resolved server-side. Preserve CLI/API output contracts. Frontend shows project display name and only uses opaque/stable ID in post-login navigation.

## Failure and boundary policy

Unknown/unmanaged project => 404; session failure => 401; unsupported capability => 409 with reason; invalid input => 400 with field errors; stale plan/digest => 409 and a new plan; provider unavailable => 503; partial write => operation record with completed/failed stages and recovery guidance. A failed preflight or dry-run never writes. Browser cannot start a process with inherited stdin or accept arbitrary command-line flags.

## Verification oracle

For each route assert parity between API and direct Core result. For every mutation assert unauthorized, cross-project, bad confirmation, stale digest, duplicate idempotency, validation failure and success behavior. Verify plan/diff causes no file, registry or journal mutation; accepted operation records actor/project/plan/result. Browser coverage includes project empty/loading/error, forms, diff review, confirmation, partial failure, keyboard operation and 320px reflow.

## Decision ledger

- Resolved: typed Core calls only; no shelling out or web-only business logic.
- Resolved: each command catalog entry is either mapped here, another package, or explicitly CLI-only.
- Deferred: TTY-bound agent takeover and any operation whose existing Core contract cannot be safely invoked without terminal input; catalog remains CLI-only until a typed contract exists.
- No blockers; unsupported existing Core capabilities must be marked visibly, never stubbed.

## Requirement traceability

| Requirement | Design decision / boundary | Success, failure and boundary scenarios | Task IDs | Verification oracle |
|---|---|---|---|---|
| Project-scoped typed workflows | Authenticated typed Core adapters | managed project; observed-only capability | 2.1–2.6, 3.1 | API outcome equals direct Core outcome for every mapped command |
| Plan before project writes | Side-effect-free plan and digest-bound confirmation | confirmed plan; missing/stale plan | 2.2–2.4, 3.2 | no file/registry/journal diff before valid confirm; stale digest refused |
| Isolated and safe project execution | Resolve registered root server-side; no shell strings | cross-project path; CLI-only action | 2.1–2.6, 3.2 | forbidden path and arbitrary command tests produce no writes/processes |
| Workflow outcome and partial failure visibility | Journal-backed operation status | partial failure; stale evidence | 2.2–2.6, 3.3 | operation stages, timestamps, stale states match Core/journal |
