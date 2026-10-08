# Design: Portfolio contract size split

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge`
  integration test target `portfolio_contract`. No sibling
  touched.
- **Module changed:** `tests/portfolio_contract.rs`
  (1,261 lines) → `tests/portfolio_contract/` (directory with
  `main.rs` + two focused submodules).
- **Modules reused unchanged:** every `src/` file and every other
  `tests/` file. The crate's `mod ...;` lines in `src/lib.rs` and
  `src/main.rs` do not change; only the filesystem shape of this
  one test target flips from a single `*.rs` file to a `*.rs/`
  directory.
- **Must NOT change:** any public symbol, journal column, route,
  CLI argument, catalog row, env var, `--version` output, or test
  name. The 23 `#[test]` functions move to submodules with their
  bodies verbatim.

## 2. Language and runtime

- Rust 2021, `rustc 1.87` floor. Build: `cargo build`. Test:
  `cargo test --test portfolio_contract` before and after,
  counts identical (23 passed / 0 failed / 0 ignored).
- Linux; no platform-specific code added or moved.

## 3. Ownership and shared code

The target follows the working `tests/kit_contract/`,
`tests/gate_contract/`, and `tests/supervised_agent_contract/`
precedents:

```
tests/<name>.rs        →  tests/<name>/
                           ├── main.rs        (helpers, `mod` decls, file doc)
                           ├── <area_1>.rs    (one group of `#[test]`)
                           ├── <area_2>.rs
                           └── ...
```

`main.rs` keeps the file-level doc comment, every shared `fn ...`
helper (CLI helpers plus the HTTP `drive` / `api_request` /
`api_json` / `seed_two_projects` seed fixtures), and the
`mod <area>;` declarations. Each submodule contains the related
`#[test]` functions and opens with `use super::*;`. Cargo
discovers the test target the same way
(`--test portfolio_contract`); every test still runs
identically. Total test count is unchanged. The only mechanical
change is the wrapping `pub(crate)` visibility (a Rust integration
test crate is its own root, so `pub(super)` would refer above the
crate root) and the `mod <area>;` declarations in `main.rs`.

For `tests/portfolio_contract/` (CLI vs HTTP per the parent
roadmap §9):

| Submodule | Concern | Tests |
|---|---|---|
| `main.rs` | file doc, shared helpers, seed fixtures, mod declarations | — |
| `cli.rs` | migration, CLI help/shape, user-owned metadata, relations, goals, evidence snapshots, compatibility | 17 |
| `http.rs` | API contract through the authorization boundary, registry-level portal projection | 6 |

Test-to-submodule assignment (names unchanged, bodies verbatim):

- `cli`:
  `pre_change_registry_migrates_forward_without_losing_rows`,
  `opening_twice_keeps_the_portfolio_schema_idempotent`,
  `help_advertises_every_portfolio_operation`,
  `tag_and_review_persist_with_project_identity_and_timestamp`,
  `unknown_project_is_a_typed_not_found_that_changes_nothing`,
  `duplicate_tag_leaves_one_link_and_repeated_review_keeps_history`,
  `invalid_tag_name_and_unknown_vocabulary_are_typed_refusals`,
  `dependency_relation_is_idempotent_and_shown_in_both_project_views`,
  `goals_are_unique_and_link_projects`,
  `fresh_evidence_is_stored_with_its_source_and_revision`,
  `expired_evidence_reads_stale_and_unavailable_never_reads_healthy`,
  `evidence_import_refuses_malformed_payload_and_time`,
  `evidence_payload_is_redacted_before_it_is_stored`,
  `evidence_import_can_read_a_fixture_file_and_bounds_it`,
  `snapshots_are_append_only_and_never_rewritten`,
  `user_metadata_is_independent_of_external_state`,
  `existing_registry_contracts_stay_compatible`.
- `http`:
  `api_portfolio_round_trip_answers_through_the_authorization_boundary`,
  `api_portfolio_mutation_without_a_session_persists_no_change`,
  `api_portfolio_session_for_another_project_is_refused`,
  `api_portfolio_malformed_payloads_are_typed_bad_requests`,
  `api_portfolio_read_of_an_unknown_project_is_not_found`,
  `portfolio_row_evidence_summary_is_an_honest_absence`.

## 4. User experience and interface

`UI/UX: N/A`. No CLI change, no API change, no frontend change,
no journal change, no env change. The only observable difference
is the file-vs-directory shape on disk, which the operator never
sees.

## 5. Behavioral model

None — this is a pure move. Each function body is copied
verbatim. The only mechanical changes are the `pub(crate)`
visibility on shared helpers, the `mod <area>;` declarations in
`main.rs`, and the `use super::*;` imports the submodules need
to reach shared helpers (per §10 verbatim-move rules).

## 6. Contract and compatibility

- No new or modified public symbol. `cargo doc` output, the
  `forge --help` output, every `forge <verb> --help`, every
  `GET /v1/admin/...` envelope, every catalog row, every
  share/publication record, every registry/journal row, every
  CLI exit code — all byte-identical.
- No `cargo test` regression:
  `cargo test --test portfolio_contract` reports
  23 passed / 0 failed / 0 ignored both before and after.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| A `main.rs` helper misses `pub(crate)` | `cargo test` fails compile; fixed before commit |
| A submodule imports a name `main.rs` did not export | `cargo test` fails compile; fixed before commit |
| The split changes the run order of `#[test]` functions | `cargo test` runs parallel by default; no assertion relies on order |
| A submodule still exceeds the cap | Not the case here (largest is ~800 lines); otherwise split further |

## 8. Verification oracle

- **The affected target:**
  `cargo test --test portfolio_contract` runs the same 23
  tests as before, 23 passed / 0 failed / 0 ignored.
- **No `src/` change:** `git status` shows only `tests/` and
  the OpenSpec package; the Gate `source-file-size` count is
  unchanged (gate evaluates `src/`; this change is tests-side
  only — stated in HANDOFF evidence).
- **Full pre-archive suite:** `cargo fmt`, `cargo fmt --check`,
  `cargo build`, `node scripts/check-openspec-change-names.mjs`,
  `openspec validate --all --strict --no-interactive`,
  `git diff --check`, `forge gate --dry-run` + `forge gate`.
- A task box is checked only with the command output for its
  assertion.

## 9. Follow-on roadmap (one file per future change)

This change is file 3 of the grind after the archived
`source-file-size-remediation` first slice (`tests/kit_contract/`
done), file 1 (`tests/gate_contract/` done), and file 2
(`tests/supervised_agent_contract/` done). The remaining
oversized files stay as pinned in the parent
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9; none of them is part of this change. The next implementer
picks the next single file from that table without re-deciding
the strategy.

## 10. Decision ledger

- **Resolved:** convert `tests/portfolio_contract.rs` to a
  directory in this change. Test files have no `pub` boundary, so
  the move is a pure refactor and the
  `cargo test --test portfolio_contract` run is the
  per-file oracle.
- **Resolved:** group submodules CLI vs HTTP per the parent
  roadmap (CLI binary surface vs HTTP authorization-boundary
  contract plus the registry-level portal projection), with all
  helpers and seed fixtures in `main.rs`.
- **Resolved:** keep every function body verbatim. No body
  change; no "while I'm here" cleanup (parent §10 rule).
- **Resolved:** do not change the test count. Every test runs
  once, in the same target, with the same name (parent §10 rule).
- **Blockers:** none.
