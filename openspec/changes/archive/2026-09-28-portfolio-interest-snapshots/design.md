# Design: Privacy-safe interest snapshots

## Implementation boundary

Repository `/home/paul/code/forge`; Rust/SQLite domain module and admin API/CLI.
Inspect `external-planes-analytics`, project authorization, migrations, and
existing audit/operation types. Add import validation, persistence, comparison
queries, and tests. Do not add collectors to products or scripts to Hugo.

## Language and runtime

Rust, SQLite, existing Forge HTTP/CLI and test toolchain. Adapters provide
already-aggregated records over a generic interface; Forge does not call a
product database directly.

## Ownership and shared code

Forge owns snapshots and portfolio comparisons. Analytics providers own
collection and identity policy. Products own raw workflow, user, and payment
data. A future shared snapshot schema belongs in `platform-contracts`; no
runtime shared library is introduced here.

## Behavioral model

Each snapshot has `project_id`, `window_start`, `window_end` (UTC, half-open),
`source`, `source_revision`, `privacy_mode`, non-negative aggregate metrics,
`received_at`, and importer actor. Allowed metric keys are explicitly named:
`unique_visitors`, `completed_public_workflows`, `returning_visitors`,
`outbound_cta_clicks`, and `paid_interest_events`; values are counts only.

Snapshots are immutable. Exact duplicate identity is idempotent; overlapping
windows from the same source are rejected unless the source revision declares a
replacement. Comparisons require matching metric definitions and windows; Forge
does not add counts from overlapping windows.

## Contract and compatibility

Import accepts versioned JSON fixture/schema records and returns per-record
accepted/rejected results without echoing sensitive values. Admin reads are
authorized by existing Forge project access. Retention defaults to aggregate
records only; raw events are never accepted.

## Failure and boundary policy

- Unknown metric, negative count, invalid timezone/window, identity field, or
  raw URL query: reject record.
- Unauthorized source or importer: deny operation.
- Stale snapshot: retain with `stale` provenance; never present as current.
- Duplicate: return existing record as idempotent success.
- Provider unavailable: no new record; existing evidence remains visible.
- Empty period: valid zero snapshot only when source declares it measured the
  complete period.

## Verification oracle

Tests prove schema validation, authorization, immutability, duplicate handling,
overlap rejection, stale labeling, metric allowlist, no identity persistence,
comparison correctness, and safe error responses. Fixtures include one valid
snapshot, each invalid field class, a duplicate, an overlap, and an empty
measured window.

## Decision ledger

- Counts are decision support, not a promise of business attribution.
- `paid_interest_events` means an aggregate signal emitted by a product; it is
  not a payment record and cannot authorize access.
- Product-native monetization is a follow-up package for one selected product.
- No unresolved design blocker remains; provider-specific credentials and
  collection policy stay outside Forge.
