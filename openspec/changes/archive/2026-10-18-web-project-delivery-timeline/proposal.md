# Proposal: Unified read-only project delivery timeline

## Why

Operators currently correlate hosted CI observations, local Gate evidence, releases, deployment and delivery journal operations, and post-publish enrollment by project and revision. The separate views obscure sequence and partial failure.

## What Changes

Add a session-gated read-only project timeline API and a Delivery page timeline. It combines existing persisted records into source-labelled events with immutable source identifiers, revision, stage/owner, source-specific status, timestamp, and safe source link where one exists. It reports coverage limits and does not synthesize an overall healthy/green result.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `web-project-delivery-timeline` | Operators can inspect one project's joined delivery history | Forge / Rust and browser JavaScript | Session-gated `GET /v1/admin/projects/{id}/delivery/timeline`, `forge-project-delivery-timeline/1.0`, Delivery page | Existing hosted CI, Gate, release, deploy, and operation stores | API contract and authenticated browser flow show events in descending time with source identity and no aggregate status |

This is the smallest independently verifiable outcome: the API projection and its same-page UI share the same event contract and acceptance journey. No second state machine, schema, or external project is introduced. GitHub completeness remains limited to authenticated observations Forge received; the timeline must disclose that limit.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge hosted CI, Gate, release, deploy and delivery records | `src/hosted_ci/store.rs`, `src/gate/engine.rs`, `src/release/engine/operations.rs`, `src/deploy/engine/stages.rs`, `src/delivery/projection.rs` | Existing owner-specific persisted records and read APIs | No common event projection; timestamps/statuses have source-specific meaning | Forge; existing records remain authoritative | **keep local** |
| Sibling projects | No sibling source is required for this read model; the existing CI/CD model identifies GitHub and provider ownership but no sibling timeline consumer | None applicable | Cross-project extraction would add a dependency for one Forge portal view | Forge owns its portal projection; GitHub/OpenPanel/Hermora remain external authorities | **keep local** |

## User Experience and Interface Impact

Forge operators choose a registered project on the existing Delivery page and use a “Refresh timeline” action. The timeline is newest-first and groups readable source, stage, source-specific status, time, revision, source ID, and an optional external link. Loading, no-events, partial-source, API error, and authenticated success states are required. At narrow widths rows reflow into labelled stacked entries; keyboard users reach refresh and source links in logical order; live status announces load completion without moving focus. Browser verification covers desktop and 390px viewport plus keyboard access and contrast.

## BFS Impact Map

- **Users/flow:** authenticated Forge operator; Delivery page project selection → refresh → inspect source-linked events.
- **Contracts:** add timeline response contract; no request body, configuration, or persistence migration.
- **Data/integrations:** read latest local Gate evidence, bounded hosted CI observations, persisted release/deploy records, and relevant project operation rows. No provider, GitHub API, shell, or mutation.
- **Callers:** admin router/handler and `frontend/app.js`; no CLI/MCP caller is added.
- **Failure/boundaries:** project ID remains server-resolved; each available source is represented independently; malformed/unavailable source data is reported as partial/unavailable rather than green; path-bearing fields and unsafe URLs are excluded.
- **Tests:** API/auth contract tests, projection ordering/bounds tests, browser interaction and responsive/accessibility checks.
- **Compatibility/security:** additive authenticated GET route; strict HTTPS GitHub run URL allowlist; DOM text rendered with text nodes; no local absolute paths or provider secrets.
- **Unaffected:** workflow generation, Gate policy, delivery preconditions, source-owned status semantics, storage schemas and release/deploy execution.
- **Shared concerns:** no suitable sibling owns a Forge portal projection; keeping this adapter local preserves ownership and avoids runtime coupling.

## Capabilities

- Add `web-project-delivery-timeline` for the read-only joined project history and its Delivery page experience.

## Non-goals

- Polling GitHub, OpenPanel, or Hermora; claiming complete remote histories; adding common pipeline IDs or persistence; changing Gate/delivery promotion rules; computing one overall CI/CD health result; enabling writes or retries from timeline entries.
