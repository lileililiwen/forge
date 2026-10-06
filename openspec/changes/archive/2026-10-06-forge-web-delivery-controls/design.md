# Design: forge-web-delivery-controls

## Implementation boundary

Forge Rust 2021 Core/API handlers and standalone frontend; reuse Git, mirror, docs, release, deploy, publish, delivery, GitHub and Studio modules. Respect OpenPanel and Hermora adapter contracts. Do not implement sibling capabilities or alter CLI behavior.

## Language and runtime

Rust 2021, current API/static Rust web listener, SQLite operation journal and browser HTML/CSS/JS. No Python or generic process-execution endpoint. Verify with existing Rust format/build plus focused provider/Core tests and API/browser end-to-end tests.

## Ownership and shared code

Forge owns orchestration and user confirmation. Existing Core owns local/remote operation semantics; configured adapters own provider authentication and external execution. OpenPanel owns staged deploy control plane; Hermora owns post-publication site operations. Interoperate only through versioned artifacts/adapters.

## Behavioral model

Every mutating workflow follows `select managed project → preflight → immutable plan → review effects → confirm exact digest → enqueue typed operation → poll by operation ID → reconcile terminal result`. Confirmation is bound to global actor, project, operation type, target/revision, plan digest and expiry. Commit, push, mirror, publish, release, deploy, delivery promotion, GitHub propose, docs translation and Studio refine each expose their own typed form and plan. Provider probing or external writes never occur on page load. Operation status distinguishes queued/running/succeeded/failed/partial/unknown/reconciliation-required. Safe retries reuse idempotency key; ambiguous remote outcomes first reconcile.

## Contract and compatibility

Add typed `/v1/admin/operations/{family}` plan/execute/status routes and operation-family resources. Plan returns `plan_id`, `digest`, expiry, project, target/ref, affected paths/resources, provider, side effects, reversible/irreversible classification and preconditions. Execute requires exact digest plus idempotency key and accepted effects. Existing CLI/MCP Core operations and journal schema remain authoritative; API routes never accept shell strings or provider secrets. `api serve`, `web serve`, `mcp` and low-level contract tools are cataloged CLI-only unless a separate product workflow exists.

## Failure and boundary policy

Expired/mismatched plan => 409 and new preflight; missing explicit confirmation => no enqueue; unauthorized/observed-only project => deny; disabled provider => blocked; timeout before dispatch => failed safe retry; timeout after dispatch => unknown/reconciliation-required; partial external write => partial with completed stages and recovery path; rollback failure => preserve both outcomes and block blind retry. Secrets and raw credentials never enter API response, browser storage, journal detail or logs.

## Verification oracle

For each operation family prove dry-run performs no write, plan digest binds reviewed state, missing/wrong confirmation creates no operation, successful execution returns an ID matching journal evidence, idempotent retry does not duplicate effect, and timeout/partial failure enters correct state. Exercise provider fixtures and explicit live sandbox only when enabled. Verify OpenPanel/Hermora handoffs use their published contracts and browser has no direct sibling/runtime coupling.

## Decision ledger

- Resolved: no generic command execution; one typed resource per operation family.
- Resolved: confirmation names exact external effects and plan digest.
- Resolved: unknown remote outcome must reconcile before retry.
- Deferred: unsupported provider-specific forms remain cataloged as unavailable/CLI-only rather than approximated.
- No blockers.

## Requirement traceability

| Requirement | Design decision / boundary | Success, failure and boundary scenarios | Task IDs | Verification oracle |
|---|---|---|---|---|
| Reviewable typed operation plans | Per-family immutable Core plans | plan success; blocker/provider failure | 2.1–2.3, 3.1 | dry-run leaves external and local state unchanged |
| Exact confirmation and operation tracking | Digest, actor, project, expiry and idempotency bound | valid confirm; stale/altered confirm | 2.1–2.4, 3.1 | mismatch dispatch count zero; success ID matches journal |
| External outcome reconciliation | Unknown/partial states cannot blindly retry | provider timeout; partial write | 2.4, 3.2 | fixture state machine records reconciliation and recovery |
| Credential and execution isolation | provider credentials stay server-side; no shell input | shell text; missing credential | 2.1–2.5, 3.1 | secret redaction and no generic process route |
