# Design: forge-web-project-fleet

## Implementation boundary

Extend Forge's Rust 2021 API read model and separate static frontend. Inspect `src/api/admin.rs`, `src/api/ui/data.rs`, `src/fleet/mod.rs`, `src/publish/inventory.rs`, registry identity code and `frontend/`. Keep JSON API and browser transport separate; all HTML/CSS/JS stays in `frontend/`. Do not modify external inventory providers or project mutation semantics.

## Language and runtime

Rust 2021, MSRV 1.87, existing blocking HTTP/1.1 API and SQLite registry, browser-native HTML/CSS/JS. No new runtime service or framework. Verify with `cargo fmt --check`, `cargo check --all-targets`, focused Rust tests, `openspec validate --all --strict --no-interactive`, and browser/API smoke using the documented separate API and web listeners.

## Ownership and shared code

The API owns aggregation and authorization. Reuse the existing fleet/inventory validators through project-local adapters; do not extract a sibling library. `frontend/` owns rendering and user interactions. External source configuration remains explicit and versioned.

## Behavioral model

Build each response as a stable union of (1) the Forge-self record, (2) local registered project rows, and (3) entries from the explicitly configured external source. Merge on declared canonical identity only. Retain separate source records when identities conflict; label the conflict rather than choosing a winner. Sort by normalized display name then stable identity. Mark each row `managed`, `observed`, or `self`; only managed rows expose Forge operation links. Self appears once even if also registered and carries `self=true` plus its registry source reference.

| Source outcome | API result |
|---|---|
| Source healthy and empty | Empty source state; other sources and Forge row remain visible |
| Source stale | Rows remain visible with `stale` and observation time |
| Source missing/malformed/unavailable | Source-level state and safe reason; healthy sources remain visible |
| Identity collision | Preserve both source records with conflict marker; disable ambiguous mutation links |
| Global session absent/expired | 401; no fleet or source details |

## Contract and compatibility

Replace the current projects response with a versioned envelope containing `projects`, `sources`, `summary`, `observed_at`, and `contract`. Project fields: `identity`, `name`, `profile`, `state`, `source`, `source_ref`, `management`, `is_self`, `freshness`, `updated_at`, `capabilities`, `evidence`. Keep existing fields during migration or version the contract before removing them. External absolute paths are never serialized. Existing `/v1` and project OIDC contracts are unchanged.

Configuration accepts explicit inventory source and freshness threshold through the established Forge settings/CLI configuration path; no guessed default path and no sibling scan. The source is read-only and bounded using existing validator limits.

## Failure and boundary policy

An unconfigured optional source is `unconfigured`, not an error. A configured but invalid source is `unavailable` with a redacted reason. One failed source does not erase other sources. Duplicates never silently collapse. Empty sources are distinct from unavailable sources. Database failure returns 503 without partial unauthorized data. Source refresh must have a bounded timeout and response size; no request triggers external writes.

## Verification oracle

Fixtures cover zero local projects, multiple local projects, multiple external entries, Forge registered and unregistered, overlap, duplicate conflict, stale source, malformed source, missing source, and unauthorized request. Assert every valid declared entry appears exactly once per identity/source, all failures are represented, no external path leaks, and only managed rows expose operation capabilities. Browser checks assert source filters, badges, search, self row and accessible partial/empty states.

## Decision ledger

- Resolved: no automatic discovery; source selection must be explicit.
- Resolved: Forge itself is included even when absent from the project registry.
- Resolved: conflicting identities are surfaced, never silently deduplicated.
- Deferred to `forge-web-command-catalog`: mapping command categories and actions.
- No blockers.

## Requirement traceability

| Requirement | Design decision / boundary | Success, failure and boundary scenarios | Task IDs | Verification oracle |
|---|---|---|---|---|
| Complete authenticated fleet aggregation | Aggregation in API fleet read model | all sources/self; self absent from registry; anonymous denied | 2.2–2.4, 3.3 | fixture asserts complete distinct rows and 401 has no data |
| Explicit sources and truthful partial states | explicit bounded adapters; per-source status | unavailable, stale, unconfigured, no scan | 2.1–2.3, 3.2 | source matrix and no-path-leak assertions |
| Identity conflicts and management boundaries | provenance-preserving canonical identity | duplicate conflict; observed-only project | 2.2–2.4, 3.3 | both records retained and mutation capabilities omitted |
