# Design: Gate contract size split

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge`
  integration test target `gate_contract`. No sibling touched.
- **Module changed:** `tests/gate_contract.rs` (1,157 lines) →
  `tests/gate_contract/` (directory with `main.rs` + five focused
  submodules).
- **Modules reused unchanged:** every `src/` file and every other
  `tests/` file. The crate's `mod ...;` lines in `src/lib.rs` and
  `src/main.rs` do not change; only the filesystem shape of this
  one test target flips from a single `*.rs` file to a `*.rs/`
  directory.
- **Must NOT change:** any public symbol, journal column, route,
  CLI argument, catalog row, env var, `--version` output, or test
  name. The 28 `#[test]` functions move to submodules with their
  bodies verbatim.

## 2. Language and runtime

- Rust 2021, `rustc 1.87` floor. Build: `cargo build`. Test:
  `cargo test --test gate_contract` before and after, counts
  identical (28 passed / 0 ignored).
- Linux; no platform-specific code added or moved.

## 3. Ownership and shared code

The target follows the working `tests/kit_contract/` precedent:

```
tests/<name>.rs        →  tests/<name>/
                          ├── main.rs        (helpers, `mod` decls, file doc)
                          ├── <area_1>.rs    (one group of `#[test]`)
                          ├── <area_2>.rs
                          └── ...
```

`main.rs` keeps the file-level doc comment, every shared `fn ...`
helper, the `GateRun` struct, and the `mod <area>;` declarations.
Each submodule contains the related `#[test]` functions and opens
with `use super::*;` (plus `use forge::gate::GATE_CONTRACT_VERSION;`
where the verdict-parity test needs it). Cargo discovers the test
target the same way (`--test gate_contract`); every test still
runs identically. Total test count is unchanged. The only
mechanical change is the wrapping `pub(crate)` visibility (a Rust
integration test crate is its own root, so `pub(super)` would
refer above the crate root) and the `mod <area>;` declarations
in `main.rs`.

For `tests/gate_contract/`:

| Submodule | Concern | Tests |
|---|---|---|
| `main.rs` | file doc, shared helpers, mod declarations, `GateRun` struct | — |
| `documented_help.rs` | help surface, dry-run rehearsal, usage refusals | 5 |
| `passing.rs` | passing runs, freshness lifecycle, registry identity, env override, verdict parity, evidence persist/read | 7 |
| `blocked.rs` | blocked documents, contradiction downgrade, redaction, evidence refusals on blocked/unknown-field/contradiction/count | 7 |
| `review_required.rs` | review-required fixture blocks and journals | 1 |
| `unknown_runtime.rs` | unresolvable/unknown runtimes, document-absent runs, timeouts, unknown targets, evidence absent/unavailable/malformed | 8 |

Test-to-submodule assignment (names unchanged, bodies verbatim):

- `documented_help`: `gate_help_surface_documents_the_verbs_and_bounds`,
  `dry_run_previews_the_plan_without_persisting_or_journaling`,
  `status_flag_conflicts_and_extra_positionals_refuse_before_work`,
  `evidence_dry_run_and_timeout_flags_refused`,
  `evidence_status_rejects_extra_positionals`.
- `passing`: `passing_run_persists_bound_evidence_journals_done_and_exits_zero`,
  `status_reports_absent_then_fresh_then_stale`,
  `registered_project_journals_under_its_registry_identity`,
  `env_override_beats_the_declaration_and_runs_exactly_that_binary`,
  `gate_verdict_surface_byte_identity_with_evidence_command`,
  `evidence_command_runs_and_persists_record`,
  `evidence_status_reads_persisted_record`.
- `blocked`: `blocked_document_is_evidence_not_failure`,
  `contradictory_pass_document_downgrades_never_fabricates_a_pass`,
  `secrets_and_host_paths_never_escape_the_redaction_pipeline`,
  `evidence_export_refused_on_blocked_state`,
  `evidence_export_refused_on_unknown_field`,
  `evidence_export_refused_on_publication_contradiction`,
  `evidence_refused_count_in_record`.
- `review_required`: `review_required_fixture_blocks_and_journals`.
- `unknown_runtime`: `missing_runtime_lists_attempts_and_preserves_prior_evidence`,
  `unknown_declared_runtime_refuses_naming_the_declaration`,
  `real_run_without_a_document_is_unavailable_never_a_pass`,
  `timeout_bounds_and_hangs_refuse_before_or_within_the_bounded_wait`,
  `unknown_target_refuses_before_any_invocation`,
  `evidence_status_absent_without_prior_run`,
  `evidence_export_unavailable_when_runtime_absent`,
  `evidence_unavailable_on_malformed_json`.

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
- No `cargo test` regression: `cargo test --test gate_contract`
  reports 28 passed / 0 ignored both before and after.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| A `main.rs` helper misses `pub(crate)` | `cargo test` fails compile; fixed before commit |
| A submodule imports a name `main.rs` did not export | `cargo test` fails compile; fixed before commit |
| The split changes the run order of `#[test]` functions | `cargo test` runs parallel by default; no assertion relies on order |
| A submodule still exceeds the cap | Not the case here (largest is ~300 lines); otherwise split further |

## 8. Verification oracle

- **The affected target:** `cargo test --test gate_contract`
  runs the same 28 tests as before, 28 passed / 0 ignored.
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

This change is file 1 of the grind after the archived
`source-file-size-remediation` first slice (`tests/kit_contract/`
done). The remaining oversized files stay as pinned in the
parent
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9; none of them is part of this change. The next implementer
picks the next single file from that table without re-deciding
the strategy.

## 10. Decision ledger

- **Resolved:** convert `tests/gate_contract.rs` to a directory
  in this change. Test files have no `pub` boundary, so the move
  is a pure refactor and the `cargo test --test gate_contract`
  run is the per-file oracle.
- **Resolved:** group submodules by scenario family per the
  parent roadmap (documented-help / passing / blocked /
  review-required / unknown-runtime), distributing the
  evidence-export tests by their behavior (persist/read →
  passing, refusals → blocked, absent/unavailable →
  unknown-runtime, flag/arg refusals → documented-help).
- **Resolved:** keep every function body verbatim. No body
  change; no "while I'm here" cleanup (parent §10 rule).
- **Resolved:** do not change the test count. Every test runs
  once, in the same target, with the same name (parent §10 rule).
- **Blockers:** none.
