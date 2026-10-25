# Tasks: studio-journal-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the Studio Core: `src/studio/state.rs` (the writers, `StudioJournalKind`), `src/studio/preview.rs` (the preview writers), `src/studio/mod.rs`.
- [x] 1.3 Read the callers: `src/cli/feature.rs` (`cmd_studio_refine`), `src/api/handlers_studio.rs`, `src/api/lifecycle_exec/studio.rs`.
- [x] 1.4 Read the canonical capability: `openspec/specs/site-studio-preview-refinement/spec.md`.
- [x] 1.5 Read the existing tests: `tests/studio_cli_contract.rs`, `tests/studio_api_contract.rs`, `tests/studio_preview_contract.rs`.
- [x] 1.6 Reproduce the defect on the current tree and record the journal row it wrote.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: a Studio journal row names the action the Studio Core performed.
- [x] 2.2 Decide where the rule lives: the Studio Core, read by every writer.
- [x] 2.3 Decide the options and reject the ones that claim a completion the Core never performed, or that promise work nobody scheduled.
- [x] 2.4 Decide the untouched surfaces: the session record, the responses, the spec-save and preview rows, the refusals.

## 3. Implement

- [x] 3.1 `src/studio/state.rs`: the rule, and the spec-save and refinement writers read it.
- [x] 3.2 `src/studio/preview.rs`: the preview writers read it.
- [x] 3.3 `tests/studio_journal_verdict_truth_contract.rs`: the refinement row, the spec-save row, the preview rows and the refusals.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A recorded refinement journals `recorded`, never `done`.
- [x] 4.4 A saved spec still journals `done`, and the preview rows still journal the observed state.
- [x] 4.5 The refusals, the session record and the responses are unchanged.
- [x] 4.6 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.7 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.8 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 2400` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [x] 6.1 Archive without `--skip-specs`, so the modified capabilities are promoted.
- [x] 6.2 Update the modified capabilities' purposes and replace the new capability's `TBD` Purpose by hand.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

### The reproduced defect

```
$ forge --registry /tmp/studio-probe/registry.db studio refine studio-probe \
    --expected-revision r1 --request "make the hero bigger"
$ sqlite3 /tmp/studio-probe/registry.db \
    "SELECT kind, project_id, state, detail FROM operations WHERE kind = 'studio.refine'"
('studio.refine', 'studio-probe', 'done', 'app_revision=r2 request=make the hero bigger selected_files=(none)')
```

The row said `done` for a request that was only validated, recorded
and journalled. The Core's own doc comment says "the agent-adapter
runtime hook is a follow-up", and the CLI's own help says "Read-only at
the file level — no editor patch is applied in this cycle". The row
contradicted both.

### The change

| Surface | Before | After |
|---|---|---|
| `src/studio/state.rs` refinement row | `"done"` | `StudioAction::RefinementRecorded.journal_state()` → `recorded` |
| `src/studio/state.rs` spec-save row | `"done"` | `StudioAction::SpecSaved.journal_state()` → `done` |
| `src/studio/preview.rs` rows | `state.label()` | `StudioAction::PreviewObserved(state).journal_state()` → the same observed state |
| `src/studio/state.rs` | — | the rule: `StudioAction` and its `journal_state()` |
| `src/studio/mod.rs`, `src/studio/state.rs` module docs | — | both name the rule |

The row after the change:

```
('studio.refine', 'studio-probe', 'recorded', 'app_revision=r2 request=make the hero bigger selected_files=(none)')
('studio.spec.save', 'studio-probe', 'done', 'spec_revision=r1 app_revision=r1 app_schema_major=1 profile=react-web')
```

### The checks

| Check | Result | Evidence |
|---|---|---|
| The reproduced defect | reproduced before the change | The row above: `('studio.refine', 'studio-probe', 'done', ...)`. The state was the literal `"done"` in `src/studio/state.rs`. |
| The one rule | PASS | `StudioAction::journal_state()` in `src/studio/state.rs` is the only decision, and all six Studio writers call it: the spec-save writer, the refinement writer and the four preview writers. |
| A recorded refinement is not a performed refinement | PASS | `a_recorded_refinement_journals_a_recorded_request`: the row's state is `recorded`, never `done`, and its detail still carries the new `app_revision`, the request and the selected file list. |
| The transport records the same row | PASS | `the_cli_refine_transport_records_the_same_row` drives `forge studio refine` through the CLI and asserts the row is `recorded`. |
| A saved spec is still a completed save | PASS | `a_saved_spec_still_journals_a_completed_save`: the `studio.spec.save` row is still `done`. |
| A preview row still names the observed state | PASS | `a_preview_row_still_names_the_observed_state` drives `start_preview` and `stop_preview` with the in-process `FakeRunner` and asserts the rows are `ready` and `stopped`. |
| A refusal still writes no row | PASS | `a_refused_refinement_writes_no_row`: a stale revision is refused with `studio-revision-conflict` and no `studio.refine` row exists. |
| One rule, every action | PASS | `one_rule_names_every_studio_action` pins the rule's five values. |
| The contract test fails without the change | PASS | With the refinement writer's state put back to the literal `"done"` and the rule left in place, two of the six cases fail: `a_recorded_refinement_journals_a_recorded_request` and `the_cli_refine_transport_records_the_same_row` both read `done` where they expect `recorded`. The other four cases pass on both trees, as they should. |
| The untouched surfaces | PASS | `git diff` changes no line of `src/studio/spec.rs`, `src/cli/feature.rs`, `src/api/handlers_studio.rs` or `src/api/lifecycle_exec/studio.rs`. The session record, the `app_revision` bump, the refusals and the three transports' envelopes are unchanged. The existing Studio suites pass unchanged: `studio_cli_contract` 13 passed, `studio_api_contract` 7 passed, `studio_preview_contract` 2 passed. |
| Repository test suite | PASS | `bash scripts/check-tests.sh` — the repository Gate's own `tests` command, and the same suite CI's `test` job runs — exited 0; the 245 `test result:` lines sum to **4284 passed; 0 failed; 1 ignored**. The pre-change baseline was 243 lines and 4278 passed, so this change adds exactly the 6 tests it declares and one new test binary. |
| Repository Gate | PASS, 8/8 | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-25T10:24:00Z"` on revision `043f3014f7807430dd77b5566776932e29e09e80` (the parent commit — the implementation is uncommitted while the Gate runs), with all eight required checks — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` — in state `pass`. The rehearsal before it (`forge gate --dry-run`) resolved the same 8 required checks, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. The advisory database's fetch was resolved to the local mirror of the same cached database (`/home/paul/.cache/advisory-db-mirror.git`, commit `640bf5a9`) through `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_0=url.file:///home/paul/.cache/advisory-db-mirror.git.insteadOf` / `GIT_CONFIG_VALUE_0=https://github.com/RustSec/advisory-db.git`, and `CARGO_HTTP_PROXY=http://127.0.0.1:1` made the unreachable crates.io index update fail fast instead of waiting for the TCP timeout. Both settings applied to that one Gate process only. |
| OpenSpec, change names, whitespace | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 167 passed, 0 failed (167 items)`; `node scripts/check-openspec-change-names.mjs` → `PASS`; `git diff --check` and `git diff --cached --check` → no finding. Archived without `--skip-specs`; the new capability `studio-journal-verdict-truth` was promoted with one requirement and four scenarios, the `site-studio-preview-refinement` "Scoped refinement" requirement was modified with all three canonical scenario names kept and one scenario added, the archive's `TBD` Purpose was replaced by hand with the real one, and the archive's trailing blank lines were stripped from the canonical specs by hand. |
| Lint and formatting | PASS | `cargo clippy --workspace --all-targets --all-features` exited 0 with 39 lines starting with `warning`, the same count as the pre-change baseline, and no warning names a touched file. `rustfmt --edition 2021 --check` is clean on all four touched Rust files, and `cargo fmt --all` was never run. No pre-existing rustfmt-dirty file was touched. Every touched file stays under the 1000 physical line limit: `src/studio/state.rs` 665, `src/studio/preview.rs` 970, `src/studio/mod.rs` 120, `tests/studio_journal_verdict_truth_contract.rs` 298. |
| The worker | the owning agent wrote the change | The worker subagents in this programme have been unreliable, so the owning agent wrote this change. An `explore` subagent did the defect hunt and returned a ranked candidate list; the owning agent chose this candidate, reproduced it, and wrote the rule, the writers and the tests. |
