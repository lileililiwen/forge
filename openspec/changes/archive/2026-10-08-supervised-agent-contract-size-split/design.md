# Design: Supervised agent contract size split

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge`
  integration test target `supervised_agent_contract`. No sibling
  touched.
- **Module changed:** `tests/supervised_agent_contract.rs`
  (1,201 lines) → `tests/supervised_agent_contract/` (directory
  with `main.rs` + four focused submodules).
- **Modules reused unchanged:** every `src/` file and every other
  `tests/` file. The crate's `mod ...;` lines in `src/lib.rs` and
  `src/main.rs` do not change; only the filesystem shape of this
  one test target flips from a single `*.rs` file to a `*.rs/`
  directory.
- **Must NOT change:** any public symbol, journal column, route,
  CLI argument, catalog row, env var, `--version` output, or test
  name. The 29 `#[test]` functions move to submodules with their
  bodies verbatim.

## 2. Language and runtime

- Rust 2021, `rustc 1.87` floor. Build: `cargo build`. Test:
  `cargo test --test supervised_agent_contract` before and after,
  counts identical (29 passed / 0 failed / 0 ignored).
- Linux; no platform-specific code added or moved.

## 3. Ownership and shared code

The target follows the working `tests/kit_contract/` and
`tests/gate_contract/` precedents:

```
tests/<name>.rs        →  tests/<name>/
                           ├── main.rs        (helpers, `mod` decls, file doc)
                           ├── <area_1>.rs    (one group of `#[test]`)
                           ├── <area_2>.rs
                           └── ...
```

`main.rs` keeps the file-level doc comment, every shared `fn ...`
helper, the `Fixture` struct and its impl, and the
`mod <area>;` declarations. Each submodule contains the related
`#[test]` functions and opens with `use super::*;`. Cargo
discovers the test target the same way
(`--test supervised_agent_contract`); every test still runs
identically. Total test count is unchanged. The only mechanical
change is the wrapping `pub(crate)` visibility (a Rust integration
test crate is its own root, so `pub(super)` would refer above the
crate root) and the `mod <area>;` declarations in `main.rs`.

For `tests/supervised_agent_contract/` (per-provider scenario
groups per the parent roadmap §9):

| Submodule | Concern | Tests |
|---|---|---|
| `main.rs` | file doc, shared helpers, `Fixture`, mod declarations | — |
| `legacy_session.rs` | help surface, bundled-path pause, legacy session shape | 3 |
| `ariadex.rs` | ariadex session lifecycle and live-status mapping | 12 |
| `sisyphusfy.rs` | sisyphusfy-supervised `run-spec` verdicts and refusals | 9 |
| `native_toolchain.rs` | runtime resolution, version probe, provider routing, env precedence | 5 |

Test-to-submodule assignment (names unchanged, bodies verbatim):

- `legacy_session`:
  `help_surface_documents_supervised_provider_and_supervisor`,
  `bundled_pause_stays_unsupported_and_never_touches_the_runtime`,
  `legacy_session_file_without_backing_still_renders`.
- `ariadex`:
  `ariadex_start_delegates_and_records_backing_runtime_handle_version`,
  `ariadex_start_uninitialized_records_disconnected_never_active`,
  `pause_and_resume_delegate_to_the_real_primitives`,
  `resume_refused_from_manual_records_the_runtimes_truth_not_active`,
  `takeover_is_attach_guidance_and_never_hijacks_the_terminal`,
  `restart_stops_then_starts_in_sibling_order`,
  `stale_daemon_maps_to_disconnected_in_transitions`,
  `unknown_stored_handle_surfaces_loss_without_mutating_the_record`,
  `local_no_daemon_status_surfaces_unreachable_state_via_live_probe`,
  `malformed_status_document_maps_to_disconnected`,
  `credential_shaped_runtime_output_is_redacted_in_evidence`,
  `new_session_supersedes_through_the_sibling_stop_then_start_verbs`.
- `sisyphusfy`:
  `run_spec_sisyphusfy_verified_loop_journals_done_with_supervisor_source`,
  `run_spec_sisyphusfy_blocked_journals_partial_with_named_reason`,
  `run_spec_sisyphusfy_malformed_outcome_is_unverified_never_done`,
  `run_spec_sisyphusfy_complete_without_passing_verification_is_unverified`,
  `run_spec_without_supervisor_keeps_bundled_path_and_has_no_verdict`,
  `run_spec_absent_supervisor_refuses_and_preserves_the_session`,
  `run_spec_ariadex_session_without_supervisor_names_the_real_paths`,
  `run_spec_unknown_supervisor_is_refused_before_anything_runs`,
  `run_spec_without_tasks_file_is_spec_invalid_before_the_supervisor_runs`.
- `native_toolchain`:
  `runtime_absent_lists_resolution_attempts_and_bundled_still_works`,
  `version_probe_mismatch_refuses_best_effort_parsing`,
  `sisyphusfy_as_session_provider_is_refused_with_the_real_path`,
  `env_override_wins_over_path_for_the_runtime_binary`,
  `no_changes_started_maps_to_disconnected_because_no_daemon_runs`.

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
  `cargo test --test supervised_agent_contract` reports
  29 passed / 0 failed / 0 ignored both before and after.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| A `main.rs` helper misses `pub(crate)` | `cargo test` fails compile; fixed before commit |
| A submodule imports a name `main.rs` did not export | `cargo test` fails compile; fixed before commit |
| The split changes the run order of `#[test]` functions | `cargo test` runs parallel by default; no assertion relies on order |
| A submodule still exceeds the cap | Not the case here (largest is ~350 lines); otherwise split further |

## 8. Verification oracle

- **The affected target:**
  `cargo test --test supervised_agent_contract` runs the same 29
  tests as before, 29 passed / 0 failed / 0 ignored.
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

This change is file 2 of the grind after the archived
`source-file-size-remediation` first slice (`tests/kit_contract/`
done) and file 1 (`tests/gate_contract/` done). The remaining
oversized files stay as pinned in the parent
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9; none of them is part of this change. The next implementer
picks the next single file from that table without re-deciding
the strategy.

## 10. Decision ledger

- **Resolved:** convert `tests/supervised_agent_contract.rs` to a
  directory in this change. Test files have no `pub` boundary, so
  the move is a pure refactor and the
  `cargo test --test supervised_agent_contract` run is the
  per-file oracle.
- **Resolved:** group submodules per provider scenario group per
  the parent roadmap (legacy session / ariadex / sisyphusfy /
  native toolchain), with runtime-resolution, version-probe,
  provider-routing and env-precedence tests in `native_toolchain`.
- **Resolved:** keep every function body verbatim. No body
  change; no "while I'm here" cleanup (parent §10 rule).
- **Resolved:** do not change the test count. Every test runs
  once, in the same target, with the same name (parent §10 rule).
- **Blockers:** none.
