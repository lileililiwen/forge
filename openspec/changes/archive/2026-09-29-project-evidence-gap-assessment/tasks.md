# Tasks: project-evidence-gap-assessment

Status: implementation complete; archived 2026-09-29.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Read `src/doctor/mod.rs` and `src/spec/mod.rs`; record the existing verdict vocabulary, finding IDs, evidence attribution and remediation classes to reuse.
- [x] 1.2 Confirm the implementation-handoff gate: Rust 1.87+, `src/doctor/gaps.rs`, CLI `forge project gaps`, dependency on the catalog contract, no new dependency.
- [x] 1.3 Map each requirement and scenario to its rule, category, verdict, callers, failure cases and named tests.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement the closed category, verdict and remediation-class vocabularies and the stable finding id.
- [x] 2.2 Implement missing/stale/invalid/unavailable/unverified rules over catalog records, with profile-derived applicability.
- [x] 2.3 Add `forge project gaps` with filters and JSON/NDJSON output; redact credentials on every path.
- [x] 2.4 Reuse the doctor and specification-remediation classification rather than introducing a second routing table.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Prove assessment is read-only (no file, registry byte, table or journal write). `tests/project_gaps_cross_surface.rs::{a_gaps_run_writes_no_registry_byte_table_or_journal_row, a_gaps_refusal_writes_nothing_either, a_gaps_run_against_a_missing_registry_never_creates_it, doctor_run_writes_no_registry_byte_either}`.
- [x] 3.2 Exercise empty, duplicate, stale, unavailable, not-applicable and conflicting-source fixtures. `tests/project_gaps_cross_surface.rs::{unavailable_and_not_applicable_are_distinct_verdicts_in_every_finding, github_remains_a_permanent_unavailable_source_with_a_named_reason, an_unavailable_source_is_never_collapsed_into_a_pass, an_empty_record_yields_a_finding_per_category_with_no_collapsing, conflicting_sources_emit_distinct_findings_with_attribution, the_summary_total_counts_unfiltered_findings_not_filtered_ones, the_gaps_envelope_carries_the_contract_version_and_a_summary}`.
- [x] 3.3 Prove existing doctor findings are unchanged by the new inspector. `tests/project_gaps_cross_surface.rs::doctor_findings_are_byte_identical_before_and_after_a_gaps_run`.

## 4. Verification

- [x] 4.1 `cargo fmt --all -- --check` clean on the four files this change touched (`src/doctor/gaps.rs`, `src/main.rs`, `tests/project_gaps_contract.rs`, `tests/project_gaps_cross_surface.rs`); pre-existing drift in `src/gate/evidence.rs`, `src/portfolio/share/validation.rs`, `src/publish/fleet.rs`, `tests/gate_contract.rs`, `tests/gate_cross_surface.rs`, `tests/publish_queue_status_contract.rs` is preserved per `AGENTS.md`. `cargo build --bin forge` succeeds. `cargo clippy --all-targets` introduces zero new warnings. `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain` passes 1933 / 0 / 0. The two new suites pass 32 / 0 / 0 (`project_gaps_contract`: 18 / 0 / 0; `project_gaps_cross_surface`: 14 / 0 / 0). `node scripts/check-openspec-change-names.mjs` passes. `openspec validate --all --strict --no-interactive` passes 57 / 0. `git diff --check` is clean.
- [x] 4.2 No repair was performed and no provider was contacted: this change only reads the catalog contract (`src/catalog/{mod,record,query,source}.rs`) and the existing doctor verdict vocabulary (`src/doctor/mod.rs`); it does not touch file writers, GitHub transport, semantic generation, CI execution or deployment. Repair is the separate `project-local-remediation-plans` package.
