# Design: Unified read-only project delivery timeline

## Implementation boundary

Forge repository only. Add a handler in `src/api/admin/history.rs`, route through the existing admin history router and `Route` enum/dispatcher, and render within the Delivery page in `frontend/index.html` and `frontend/app.js`. Add API and browser contract coverage. Do not change source stores, templates, provider adapters, workflows, or sibling projects.

## Language and runtime

Rust stable (edition/toolchain declared by `Cargo.toml`) with existing serde/serde_json and SQLite-backed Registry; browser ES modules/vanilla JavaScript. Verify with focused `cargo test` integration contracts, `node --check frontend/app.js`, existing browser harness, `cargo fmt --check`, and repository Gate. No new dependency or database migration.

## Ownership and shared code

Forge owns the aggregation and portal projection. Hosted CI observations remain owned by `hosted_ci`, local Gate evidence by Gate, releases/deploys by their existing engines, and operation status by the registry/provider journal. No shared extraction: the projection serves Forge's portal and must not become a sibling runtime dependency.

## User experience and interface

Actor: signed-in Forge administrator. Entry: existing Delivery page and selected project. A “Refresh timeline” button requests the event projection. Timeline cards show source/owner, stage, source-specific status, observed/finished time, revision (short display plus full value), immutable source ID, and optional source URL. Events sort newest first, ties by stable source/type/ID order. Show an explicit scope note: hosted GitHub events are only observations received by Forge; local Gate history is latest persisted evidence; absent records are not proof of success or failure.

States: initial prompt to select project; loading with polite live text; empty with source coverage note; populated success; partial source availability with per-source explanation; unauthorized/session expiry via existing API behavior and delivery error summary; retry by refresh. No destructive or write action. At 390px entries stack into labelled rows; desktop uses compact event cards. Use semantic list/article headings, native button and links, visible focus, text nodes, contrast-compliant existing tokens, no motion requirement. Browser oracle must cover 390px and desktop, keyboard traversal, live-region status, empty/populated/partial/error states, and link safety.

## Behavioral model

One GET constructs an ephemeral snapshot; it writes no state. Sources are read independently and each source contributes zero or more events. Maximum response is 200 events total; source reads are bounded to hosted 20 observations, 200 newest project operations before kind filtering, 50 release entries, 50 deploy entries, and one latest Gate record. A release with stage outcomes contributes one event per outcome. Deterministic sort is descending RFC3339 timestamp with stable source/id tie-break; invalid/missing times sort last and remain visibly labelled unavailable. No event is collapsed by matching SHA. Each event status is exactly the owning record's status vocabulary; a release without stage outcome remains `recorded`. The payload carries per-source `available`, `count`, and `note`, plus `coverage: partial|complete`; complete means all queried local sources were readable, not that remote histories are complete.

## Contract and compatibility

`GET /v1/admin/projects/{id}/delivery/timeline` is guarded by the existing admin session middleware and project registry ID gate. Response:

```json
{"project_id":"example","contract":"forge-project-delivery-timeline/1.0","coverage":"partial","sources":[{"source":"hosted_ci","available":true,"count":1,"note":"Hosted observations received by Forge only."}],"events":[{"id":"hosted-ci:opaque-id","source":"hosted_ci","owner":"GitHub Actions","stage":"gate","status":"blocked","occurred_at":"2026-01-01T00:00:00Z","revision":"<sha>","source_id":"run 123 attempt 1","source_url":"https://github.com/owner/repo/actions/runs/123"}]}
```

All event fields except `id`, `source`, `owner`, `stage`, `status` may be null where a source lacks that evidence. The schema is additive and versioned. Existing routes are unchanged. URLs are included only for validated `https://github.com/{owner}/{repo}/actions/runs/{digits}` links from validated hosted envelopes. Absolute local paths, provider state paths, raw operation details that can contain secrets, and browser paths are excluded.

## Failure and boundary policy

Unknown project returns the existing typed 404; missing/expired session uses existing 401 behavior. Each source read failure yields an unavailable source summary and an empty contribution; other sources still render. Registry/project lookup failure remains a typed request failure. No source failure is converted to pass. Malformed timestamps remain events but sort last. Duplicate observations retain unique source IDs. Response is capped at 200 events with deterministic source quotas and includes `truncated` in source notes when capped. No retry occurs except operator-triggered refresh.

## Verification oracle

- Rust API contract verifies auth, unknown project, response contract, source scoping, no absolute paths, link allowlist, partial source handling, event order, caps, and source-status preservation using temporary registry/project fixtures.
- Browser flow signs in, selects a fixture project, validates empty/populated/partial/error rendering, source links, 390px reflow, keyboard reachability, live announcements, and text contrast.
- `node --check frontend/app.js`; targeted Rust integration tests; rustfmt; `git diff --check`; strict OpenSpec validation; `forge gate --dry-run` then bounded full `forge gate` before archive.

## Rollout and migration

Additive GET route and UI section; no migration, backfill, environment key, or rollout flag. Existing Forge instances expose the feature after binary/frontend upgrade. Existing source data remains authoritative and is read on demand.

## UI/UX applicability

Applicable: this is an operator-facing Delivery view. The interface and browser acceptance journey are part of this package and cannot be replaced by API evidence.
