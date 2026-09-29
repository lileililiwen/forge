# Tasks: github-project-metadata-adapter

Status: implementation complete. All tasks closed against the
implementation evidence recorded in the change package.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Read the provider/governance selection and redaction code plus `src/release/`; record the bounded-process, credential-reference and confirmation patterns to reuse.
- [x] 1.2 Confirm the implementation-handoff gate: Rust 1.87+, `src/github/*`, external adapter executable contract, PR-mode default, no new dependency, token from environment only.
- [x] 1.3 Map each requirement and scenario to its adapter call, normalization field, provider state, caller, failure case and named test.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement the `forge-github-metadata/0.1.0` request/response contract and bounded argument-array invocation with timeout. Implemented in `src/github/adapter.rs`: `GithubAdapter`, `observe`, `MutationMode`, `ProposeRequest`, `GithubObservationRequest`; `Command::arg` arrays pass `["observe"|"propose", ...]`; `tokio::process::Command` alternative is intentionally avoided to keep zero new deps; bounded 15s timeout via `std::process::Command::output` + caller-side wall-clock guard.
- [x] 2.2 Implement normalization into catalog records with source, revision and freshness, and the closed provider-state mapping. `src/github/normalize.rs::normalize_observation` builds the `CatalogRecord`; `evidence_state` maps the closed provider-state set (`current`/`stale`/`unavailable`/`unauthorized`/`forbidden`/`not-found`/`rate-limited`/`partial`) to `EvidenceState`. 13 unit tests cover the mapping.
- [x] 2.3 Implement PR-mode default and explicit-confirmation direct mode; refuse implicit settings changes. `src/github/adapter.rs::MutationMode` defaults to `PullRequest`; `Direct { confirmation }` requires a non-empty confirmation; the adapter must echo the same `FORGE_GITHUB_TOKEN` in the body of the call. Refusal paths exercised by `direct_mode_requires_a_non_empty_confirmation` and `direct_mode_refuses_*` tests.
- [x] 2.4 Keep tokens and response bodies out of reports; keep GitHub topics, release tags and portfolio tags separate. `clean_field` in `src/catalog/record.rs` (200-char bound + credential redactor) is applied to every emitted field. `normalize_observation` keeps `tags` empty for GitHub records; `topics` and `releases` live in the `observations` payload of the JSON document and the human report, never on the closed record. Tested by `the_portfolio_tag_list_is_always_empty`, `credential_in_description_is_redacted`, `credential_shaped_topic_is_dropped_from_the_list`, and `github_records_never_gain_topics_or_releases_on_the_record_field`.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Prove no request is sent without configuration and local-only Forge is unaffected. `missing_binary_is_typed_unavailable_with_0_bytes_of_stdout`, `missing_token_is_typed_invalid_with_0_bytes_of_stdout`, `catalog_github_source_reports_unavailable_with_no_binary_and_no_token` in `tests/github_adapter_contract.rs` prove the typed error paths. `a_github_read_writes_no_registry_byte_table_column_index_or_journal_row` in `tests/github_adapter_cross_surface.rs` proves the local registry is byte-identical (same bytes, same tables, same columns, same indexes, same project count, same operations count) after every read path.
- [x] 3.2 Exercise unauthorized, forbidden, not found, rate limited, unavailable, stale, partial and malformed-adapter boundaries against a local stub. `closed_state_vocabulary_is_stable` and `parse_state_handles_every_kebab_case_variant` cover the closed vocabulary; `unauthorized_state_is_unverified_evidence` and `rate_limited_state_is_reported_with_reset_and_no_partial_payload` cover the wire-level boundaries; `empty_repository_list_is_github_invalid`, `malformed_repository_identity_is_github_invalid`, `repository_validation_rejects_malformed_values` cover the closed set of `owner/repo` identities; `an_unknown_proposed_field_is_github_invalid` covers the closed proposed-field set.
- [x] 3.3 Prove a token never reaches any output and namespaces never merge. `a_credential_in_an_adapter_response_is_redacted` and the cross-surface `a_github_token_never_reaches_stdout_stderr_or_a_human_report` prove the token is redacted in JSON, human, NDJSON, inspect and the dedicated `forge project github observe|propose` surfaces, and that the operations journal never records it. `github_records_never_gain_topics_or_releases_on_the_record_field` proves the closed record key set never gains a GitHub-only field; the local records never receive `topics` or `releases`; the `observations` payload keeps the three namespaces separate.

## 4. Verification

- [x] 4.1 Run `cargo fmt/build/clippy/test`, the two new suites, `openspec validate --all --strict --no-interactive`, the name preflight and `git diff --check`; record the exact output. Captured below in the change package.
  - `cargo build` — success (no new warnings).
  - `cargo clippy --all-targets -- -D warnings` — 12 pre-existing baseline errors in `src/api/ui/auth.rs:166-167`, `src/gate/evidence.rs:229/425/826/827/862`, `src/portfolio/share/validation.rs:13/392`, `src/publish/fleet.rs:51`, `src/publish/mod.rs:642/644`; zero new errors from the change.
  - `cargo fmt --all -- --check` on the touched files — no diffs in the change's files (the remaining diffs are pre-existing baseline drift in `src/gate/evidence.rs`).
  - `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain` — every test passes except the pre-existing baseline `fleet_online_routes_to_local_listener_when_alethefy_is_up` failure (sandbox listener restriction, fails on the stashed baseline too).
  - `cargo test --test github_adapter_contract` — 11/11 pass.
  - `cargo test --test github_adapter_cross_surface` — 3/3 pass.
  - `cargo test --lib -- github` — 25/25 pass.
  - `node scripts/check-openspec-change-names.mjs` — `PASS`.
  - `openspec validate --all --strict --no-interactive` — 57/57 items pass.
  - `git diff --check` — clean.
- [x] 4.2 Record that no live GitHub host was contacted and no repository was mutated; all fixtures are local executable stubs. Every test plants `forge-github-metadata-adapter` as a `/bin/sh` script on a controlled `PATH`; the stub never opens a socket and only inspects the inherited environment for `FORGE_GITHUB_TOKEN`. No `https://`, `http://`, `git://` or `ssh://` URL is dereferenced outside the local stub, the local registry file and the local git working tree. `proxy` env vars (`HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY`) are removed in every test. The proposed set is never carried through a non-PR adapter path unless a `--confirm <token>` is supplied and the adapter echoes the token back, exercised in `direct_mode_refuses_*`. No mutation path produces a write; `a_github_read_writes_no_registry_byte_table_column_index_or_journal_row` proves the registry bytes, tables, columns, indexes and journal counts are byte-identical before and after. `cargo deny check` is **not run** (sandbox network restriction); this is recorded in HANDOFF rather than as a pass; the change adds zero new dependencies.
