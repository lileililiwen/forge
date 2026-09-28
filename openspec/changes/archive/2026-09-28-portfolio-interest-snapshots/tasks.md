# Tasks

## 1. BFS — Baseline and impact coverage

- [x] Map snapshot requirements to adapter, schema, authorization, migration, query, retention, and test layers.
      Evidence: the mapping lives in the module docs of `src/portfolio/interest/mod.rs` (what a
      snapshot *is*), `validation.rs` (what may enter), `compare.rs` (how it is read back),
      `src/portfolio/interest_report.rs` (the single orchestration door), and
      `src/registry/interest/mod.rs` (persistence). The three spec requirements map to the gate
      (`validate_snapshot`), the append-only store plus idempotency rule
      (`interest_insert_snapshot`), and the read model (`build_comparison`, `build_trend`).
- [x] Confirm existing external analytics adapter and window semantics; document non-overlap invariant.
      Evidence: the sibling `external-planes-analytics` runtime in `src/analytics/mod.rs` already
      refuses to sum across windows (`finalize_aggregate`, `METRIC_STATE_MIXED_WINDOWS`) and already
      carries a window vocabulary. This package reuses that *decision* but not that code: its own
      windows are UTC half-open `[start, end)`, so adjacency is allowed and overlap is refused
      unless the source declares a replacement. The invariant is stated in the `InterestSnapshot::overlaps`
      doc and proved by `half_open_windows_never_overlap_at_their_boundary` and
      `an_overlapping_window_is_refused_unless_the_source_declares_a_replacement`.
- [x] Confirm product data and payment records remain outside Forge.
      Evidence: the snapshot key set is closed (`SNAPSHOT_KEYS`, nine keys) and the metric set is a
      closed allowlist (`InterestMetric::ALL`, five counts). `PAYMENT_KEYS` and `IDENTITY_KEYS` are
      explicit refusal vocabularies, `paid_interest_events` is documented as an aggregate signal and
      is deliberately absent from `PAYMENT_KEYS`. `the_store_has_no_column_that_could_hold_a_raw_payload`
      proves the schema itself has no free-form column, so no collector, pixel or product script can
      reach the store through a field Forge did not name.

## 2. DFS — Requirement-by-requirement implementation

- [x] Add versioned snapshot schema and migration.
      Evidence: `INTEREST_CONTRACT_VERSION = "forge-portfolio-interest/0.1.0"` gates the import
      envelope (`parse_import_document`, strict on an absent or wrong contract), and
      `PORTFOLIO_INTEREST_SCHEMA_SQL` adds `portfolio_interest_snapshots`,
      `portfolio_interest_metrics` and `portfolio_interest_findings` with
      `apply_portfolio_interest_migration` — its own explicit `BEGIN IMMEDIATE` … `COMMIT` batch with
      `ROLLBACK`, deliberately separate from the portfolio and share batches.
      `an_interrupted_interest_migration_rolls_back_and_leaves_the_registry_usable` proves a failed
      batch leaves no partial table and the next open retries cleanly, and
      `a_pre_change_registry_migrates_forward_with_its_rows_intact` proves a pre-change registry
      carries forward untouched.
- [x] Add import validation, allowlisted metrics, provenance, and safe per-record results.
      Evidence: `validate_snapshot` is the single gate and checks the closed key set *before* reading
      any value, then provenance, then the metric object. `InterestWrite` carries project, source,
      source revision, the normalized UTC window, `PrivacyMode`, `Coverage`, the optional replacement
      and the canonical metric order. `import_snapshots` returns an `ImportReport` with `accepted`,
      `already_present`, `supersessions` and per-record `rejected` entries; a rejection names the
      index, field, code and reason and never the value. `every_refused_field_class_is_named_and_never_echoed`
      covers all four refusal classes plus an unknown field, and
      `an_import_records_provenance_and_reports_each_record` proves the report shape.
- [x] Add immutable/idempotent persistence and overlap checks.
      Evidence: `interest_insert_snapshot` resolves an exact identity repeat as `AlreadyPresent`
      (no second row), refuses a changed payload under an already-attested identity as a
      `portfolio-interest-conflict`, and runs the overlap rule against stored evidence before it
      opens its transaction. Only `accepted` → `superseded` ever changes, and only when the source
      names the revision it replaces (`a_declared_replacement_supersedes_the_named_revision_and_keeps_the_history`).
      `an_exact_repeat_is_idempotent_and_a_changed_payload_is_a_conflict` covers both halves.
- [x] Add admin comparison/trend queries with stale and source labels.
      Evidence: `compare_projects` and `interest_trend` in `src/portfolio/interest_report.rs`, built
      by `build_comparison` and `build_trend` in `compare.rs`. Every comparison row carries
      `source`, `source_revision`, `privacy_mode`, `coverage` and `freshness`; there is no total,
      average or rank field at any level. `freshness_label` derives `current`/`stale` from the window
      end at read time, never stores it, and reads an unreadable bound as `stale` rather than as the
      present. Both views narrow to current snapshots, so a superseded revision never answers the
      same question twice (`a_superseded_revision_never_appears_in_a_trend`).
- [x] Add unit, authorization, fixture, duplicate, overlap, and privacy tests.
      Evidence: 34 unit tests across `src/portfolio/interest::{mod,validation,compare}` and
      `src/registry/interest`; 16 `tests/portfolio_interest_cli_contract.rs`; 13
      `tests/portfolio_interest_api_contract.rs`; 10 `tests/portfolio_interest_cross_surface.rs`.
      Authorization is covered by `every_interest_route_demands_a_session` (every route including the
      reads), `a_cross_project_session_cannot_import_into_another_project` and
      `the_api_import_records_the_session_subject_as_provenance`.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify no raw event, identity, payment, credential, or hidden site collector can enter the snapshot store.
      Evidence: `the_store_has_no_column_that_could_hold_a_raw_payload` asserts the exact column set of
      all three tables and that no column name contains a payload/identity/payment/credential marker;
      `the_stored_metric_rows_are_only_allowlisted_names` reads the stored metric names back out of
      SQLite and proves each is on the allowlist. No collector is added: the package has no adapter
      command, no route that contacts a host, and `an_import_touches_no_network_and_journals_no_operation_row`
      plus `an_import_writes_no_operations_journal_row` prove the import writes only its own rows.
- [x] Verify overlapping windows are never summed and unavailable providers do not erase evidence.
      Evidence: `overlapping_windows_are_never_merged_or_double_counted` stores two sources over the
      same instants and proves the comparison returns two rows and never the sum;
      `a_comparison_never_sums_and_says_so_when_windows_differ` proves the shape carries no total
      anywhere. `a_refused_import_never_erases_stored_evidence` proves a batch whose records are all
      malformed leaves the earlier snapshots readable. A cross-source window is allowed and a
      same-source overlap is refused (`a_different_source_may_report_the_same_window`).
- [x] Verify existing analytics adapter behavior and project authorization remain compatible.
      Evidence: `the_analytics_adapter_surface_is_untouched` runs `forge analytics metrics` after an
      import and proves it still succeeds and reads none of the interest store;
      `the_private_portfolio_projection_carries_no_interest_data` and
      `the_public_share_manifest_carries_no_interest_data` prove neither sibling read model gained a
      field; `the_fleet_list_envelope_is_unchanged` proves `forge list` is unchanged. The existing
      share and portfolio contract suites still pass without edits, and no MCP tool, portal route or
      journal kind was added.

## 4. Verification

- [x] Run formatter, linter, unit/integration tests, fixture validation, and strict OpenSpec validation.
      Evidence: `cargo fmt --all -- --check` is clean for every touched file (the pre-change drift in
      `src/gate/evidence.rs`, `src/publish/{fleet,jenkins}.rs`, `src/portfolio/share/validation.rs`,
      `tests/gate_contract.rs`, `tests/gate_cross_surface.rs` and
      `tests/publish_queue_status_contract.rs` was preserved exactly); `cargo build` PASS;
      `cargo clippy --all-targets -- -D warnings` reports the same 12 pre-existing locations as the
      stashed baseline and zero new ones; `cargo test --workspace --all-targets -- --skip
      rust_scaffold_builds_and_tests_with_native_toolchain` is 78 result groups, 1772 tests, 0 failed;
      `cargo deny check` is advisories/bans/licenses/sources ok with no new dependency added;
      `node scripts/check-openspec-change-names.mjs` PASS;
      `openspec validate --all --strict --no-interactive` 50 passed, 0 failed; `git diff --check` PASS.
      Live binary smoke: the CLI import/compare/trend/show/audit flow against a two-project registry,
      and the same flow over `forge api serve` on loopback with a minted admin session, including
      401 for every interest route without a bearer and a 403 for a cross-project session.
- [x] Record provider credentials/runtime collection as external evidence, not local test success.
      Evidence: **no analytics provider was contacted and no credential was used.** Every fixture in
      this package's tests is a local JSON document or an in-process call; there is no adapter
      subprocess, no `FORGE_*_BIN` override and no network request in the interest code path. The
      `import` verb is the *consuming* half of an analytics pipeline: the provider that produces the
      aggregate, its credentials and its collection policy stay outside Forge, and this package
      claims nothing about how a figure was produced beyond the `privacy_mode` and `coverage` the
      source declared. Real provider runtime collection remains external evidence that does not exist
      yet.
- [x] Do not start product billing work until one project has reviewed aggregate evidence.
      Evidence: no billing, subscription, entitlement, CRM or revenue-attribution code was added or
      touched. `paid_interest_events` is stored as an aggregate signal and is documented as *not* a
      payment record and never a basis for granting access (`InterestMetric::PaidInterestEvents`,
      the design decision ledger, and the refusal vocabularies). Whether to start product-native
      monetization is a separate future package, gated on a project having reviewed aggregate
      evidence, and this change does not begin it.
