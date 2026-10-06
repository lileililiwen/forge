# Design: forge-web-portfolio-controls

## Implementation boundary

Forge Rust 2021 Core/API modules for portfolio, catalog, fleet, inventory, governance, analytics, provider and readiness; standalone frontend. Inspect existing command enums and canonical contracts. Keep user-owned writes in portfolio registry and source-owned evidence append-only. Do not modify sibling projects.

## Language and runtime

Rust 2021, existing SQLite and blocking JSON API, standalone HTML/CSS/JS, no new runtime. Verify formatting/build, focused module/API tests, OpenSpec strict validation and browser workflows.

## Ownership and shared code

Forge's registry owns its tags, relations, reviews, goals, allowlist and share approval. External providers own their source facts. Existing Forge adapters validate and project data. No sibling imports; adapter invocation stays bounded and optional.

## Behavioral model

Read-only operations: project catalog list/inspect/tags/languages/gaps; fleet list/status/inspect; inventory show; portfolio show/evidence/interest/readiness; governance list/status/inspect; analytics inspect/metrics; provider matrix/inspect; readiness matrix/artifact/check. Mutations: portfolio tag/relation/review/goal/share operations and governance provider use. Each mutation requires typed fields and CSRF/origin validation, records actor and timestamp, and cannot edit source-owned evidence. Provider matrix is non-live by default; live checks require a separate explicit confirmation and report `not_run` until executed. Sharing continues preview → approve exact digest → publish in the separately specified delivery package; this package may manage the allowlist/preview/approval record, but cannot publish it.

## Contract and compatibility

Authenticated `/v1/admin/portfolio/*`, `/catalog/*`, `/fleet/*`, `/inventory/*`, `/governance/*`, `/analytics/*`, `/provider/*`, `/readiness/*` are typed JSON resources. IDs and schema versions reuse canonical contracts. Responses retain per-source freshness, provenance and status vocabulary. Imported data stays append-only; no existing CLI output or `/v1` contract changes.

## Failure and boundary policy

Unknown project => 404; invalid filters => 400; unauthorized => 401/403; unavailable source => partial result with explicit status; stale => visible stale state; malformed source => source error with safe reason; provider not opted in => `not_run`; privacy threshold not met => aggregate withheld; foreign/source-owned mutation => refused. No status is converted to healthy by omission.

## Verification oracle

Assert browser values match Core outputs, source and timestamps are retained, every mutation updates only Forge-owned rows, source evidence is append-only, live provider commands make no network/process call unless explicit opt-in, interest thresholds redact low-count cohorts and stale/unknown remain visible. Test authorization and audit fields for each mutation family.

## Decision ledger

- Resolved: the package can edit Forge-owned portfolio metadata but only view imported observations.
- Resolved: live provider evidence requires an explicit separate action; landing the page performs no probes.
- Resolved: this package delivers portfolio metadata controls and truthful evidence views; the share allowlist/preview/approval/publish flow is owned by the delivery controls package.
- Deferred: user roles (single global admin only) and new provider types.
- No blockers.

## Requirement traceability

| Requirement | Design decision / boundary | Success, failure and boundary scenarios | Task IDs | Verification oracle |
|---|---|---|---|---|
| Portfolio-owned metadata controls | Existing portfolio Core writes only | review mutation; imported evidence edit denied | 2.1, 2.4, 3.2 | changed fields belong to Forge registry; source snapshot unchanged |
| Cross-project evidence views | typed source adapters preserve provenance | stale/unavailable; provider not run | 2.2–2.3, 3.1 | DTO equals Core report and page load causes no probe |
| Privacy and evidence ownership | canonical threshold and ownership checks | below-threshold aggregate; disabled provider | 2.3, 3.2–3.3 | redaction and true provider state assertions |
