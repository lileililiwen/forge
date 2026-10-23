# Tasks: docs-translate-review-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the report and its rule: `src/docs/model.rs` (`TranslateReport`, `TranslateOutcome`, `healthy_check`).
- [x] 1.3 Read the run: `src/docs/translate.rs` (`run_translate`, the note, `validate_output`, the review status).
- [x] 1.4 Read the CLI: `src/cli/release.rs` (`cmd_docs_translate`, the journal row, the exit code, `docs_translate_output`).
- [x] 1.5 Read the release lane: `src/release/engine/stages.rs` (the docs stage).
- [x] 1.6 Read the doctor's rule: `src/doctor/freshness.rs` (a needs-review derivative is a warning).
- [x] 1.7 Read the canonical capability: `openspec/specs/documentation-translation/spec.md`.
- [x] 1.8 Find every other reader of the report or of a locale's review status.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: one aggregate verdict — `complete`, `needs-review`, `partial`.
- [x] 2.2 Decide what "judged" means for a locale: translated or current.
- [x] 2.3 Decide the precedence: a failure outranks a review.
- [x] 2.4 Decide what a needs-review verdict does to the exit code and the journal state: neither.
- [x] 2.5 Decide the report's `healthy` field: true only for `complete`.
- [x] 2.6 Decide the release lane's change: the stage's note names the review, the stage's status does not change.

## 3. Implement

- [x] 3.1 `src/docs/model.rs`: the typed verdict and the rule that derives it from the outcomes.
- [x] 3.2 `src/docs/translate.rs`: the note names the verdict; the report carries the verdict.
- [x] 3.3 `src/cli/release.rs`: the exit code, the journal state and the journal detail follow the verdict.
- [x] 3.4 `src/release/engine/stages.rs`: the docs stage's note names the locale's review status.
- [x] 3.5 `tests/docs_translate_review_truth_contract.rs` (new): the cases, driven through the real CLI.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A locale that needs review: the verdict is `needs-review`, the report is not healthy, the summary names it, the exit code is 0 and the journal state is `done`.
- [x] 4.4 Every locale reviewed clean: the verdict is `complete`, the report is healthy, the exit code is 0.
- [x] 4.5 A failed locale: the verdict is `partial`, the exit code is 1 and the journal state is `partial`.
- [x] 4.6 A failed locale and a locale that needs review: the verdict is `partial`.
- [x] 4.7 The JSON document publishes the verdict.
- [x] 4.8 The release lane's docs stage names the review, and the stage's status is still `delivered`.
- [x] 4.9 The derivative, the recorded review status and the source hash are unchanged.
- [x] 4.10 The doctor's finding is unchanged.
- [x] 4.11 The contract version is unchanged.
- [x] 4.12 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.13 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.14 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 1800` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [x] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [x] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capabilities' purposes.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed.

| Check | Result | Evidence |
|---|---|---|
| 1.4 the reproduced defect | reproduced, before the change | The owning agent's own run, not re-run here: a project whose `README.md` carried the link `[the manual](https://example.com/manual)`, and a translator that rewrote it to `the manual`. `FORGE_DOCS_TRANSLATOR_BIN=<dropper> forge --registry <db> docs translate zh-CN --project <dir>` printed `  - zh-CN translated derivative=docs/README.zh-CN.md: locale \`zh-CN\` translated to \`docs/README.zh-CN.md\` at source hash \`82dbd281ea67\` (4 segment(s) translated, 0 reused)`, `      review: needs-review`, `      review-reason: link destination \`https://example.com/manual\` was altered or dropped`, and `summary: docs translate complete for project \`docs-app\`: 1 translated, 0 current`, and exited 0. The JSON document carried `"healthy": true` and the same note. |
| 4.1 `cargo build` | pass | `cargo build --workspace` exited 0; the workspace finished compiling in 1m 16s and the `forge` binary was produced. |
| 4.2 the repository test suite | pass | `bash scripts/check-tests.sh` — the repository Gate's own `tests` command, and the same suite CI's `test` job runs — exited 0; the 228 `test result:` lines sum to **4188 passed; 0 failed; 1 ignored**. The pre-change baseline, measured with the same script, was 227 lines and 4181 passed, so this change adds exactly the 7 tests it declares (5 contract cases, 2 unit tests) and exactly one test binary. |
| 3.5 the contract test | pass | `cargo test --test docs_translate_review_truth_contract` → `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. The five: `a_locale_that_needs_review_is_never_complete`, `every_locale_reviewed_clean_is_complete`, `a_failed_locale_is_partial`, `a_failure_outranks_a_review`, `the_derivative_and_its_recorded_state_are_unchanged`. |
| 4.3 the needs-review run | pass | The owning agent's own run of the real CLI after the change, with the same dropping translator: the human report printed `      review: needs-review`, `      review-reason: link destination \`https://example.com/manual\` was altered or dropped`, `verdict: needs-review` and `summary: docs translate needs review for project \`docs-app\`: 1 translated, 0 current, 1 locale(s) need review (zh-CN)`, and exited 0. The JSON document carried `"verdict": "needs-review"`, `"healthy": false` and the note `docs translate needs review for project \`docs-app\`: 0 translated, 1 current, 1 locale(s) need review (zh-CN)`. The summary no longer says "complete" and names the locale that needs review. |
| 4.4 the clean run | pass | `tests/docs_translate_review_truth_contract.rs::every_locale_reviewed_clean_is_complete`: the human report contains `verdict: complete` and a summary that says the run is complete, the JSON document carries `"verdict": "complete"`, `"healthy": true` and `"outcomes"[0]["review"] == "ok"`, the exit code is 0 and the journal row's state is `done` with `verdict=complete` in its detail. |
| 4.5 the failed run | pass | `tests/docs_translate_review_truth_contract.rs::a_failed_locale_is_partial`: the exit code is 1, the human report contains `verdict: partial`, the JSON document carries `"verdict": "partial"` and `"healthy": false`, and the journal row's state is `partial` with `verdict=partial` in its detail. |
| 4.6 a failure outranks a review | pass | `tests/docs_translate_review_truth_contract.rs::a_failure_outranks_a_review`: two locales, one translated with a dropped link and one refused by the provider, give exit code 1, `verdict: partial` in the human report, `"verdict": "partial"` in the JSON document, and the two outcomes' reviews are `needs-review` and `ok`. |
| 4.7 the published verdict | pass | The report carries the verdict as an always-present field, so `serde_json::json!({"translate": report})` publishes it: the JSON document carries `"verdict": "needs-review"` / `"complete"` / `"partial"` in the three cases above, and the human renderer prints a `verdict: <label>` line before the summary. |
| 4.8 the release lane's docs stage | pass | `src/release/engine/engine_tests.rs::docs_stage_names_the_review_and_stays_delivered` drives `run_docs_stage` with a dropping translator: the stage's note names the locale's review status (`review \`needs-review\``), and the stage's status is still `delivered`. `cargo test -p forge --lib release::engine` → `test result: ok. 20 passed; 0 failed`. |
| 4.9 the untouched derivative | pass | `tests/docs_translate_review_truth_contract.rs::the_derivative_and_its_recorded_state_are_unchanged`: the per-locale outcome still carries `"status": "translated"`, `"review": "needs-review"` and its one reason naming the dropped destination, the derivative file still carries the provider's output and the code block verbatim, and the recorded state still carries the same review status, the same reason, the 64-character source hash and the contract `0.1.0`. |
| 4.10 the doctor's finding | pass | `src/doctor/freshness.rs` is untouched: a needs-review derivative is still `FindingStatus::Warn`. The change makes the report agree with the doctor rather than replacing the doctor's rule. |
| 4.11 the contract version | pass | The report still carries `"contract": "0.1.0"`; the recorded state file still carries `"contract": "0.1.0"`. The report gains one always-present field (`verdict`); no field is removed or renamed. |
| 4.12 rustfmt | pass | `rustfmt --edition 2021 --check` is clean on all nine touched Rust files, and `cargo fmt --all` was never run. No pre-existing rustfmt-dirty file was touched. |
| 4.13 clippy | pass | `cargo clippy --workspace --all-targets --all-features` exited 0 with 39 warnings, the same count as the pre-change baseline, and no warning in a touched file. |
| 4.14 the source file line counts | pass | Every touched file is under the 1000 physical line limit: `src/docs/model.rs` 387, `src/docs/translate.rs` 731, `src/docs/docs_tests.rs` 496, `src/docs/mod.rs` 85, `src/cli/release.rs` 630, `src/release/engine/stages.rs` 652, `src/release/engine/engine_tests.rs` 875, `tests/docs_translate_review_truth_contract.rs` 436, `tests/docs_contract.rs` 673. |
| 5.1 `forge gate --dry-run` | pass | `./target/debug/forge gate --dry-run` → `dry-run: gate plan rehearsal for forge (nothing was executed, persisted or journaled)`, `rehearsal: driftwatch-gate-plan/0.1.0 (gate-plan-rehearsal) | execution mode: dry-run | nothing was executed, persisted or journaled`, `blocking policy: review_required_blocks=true | default timeout: 60000ms | source size max_lines: 1000`, `checks (8 planned):` listing `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` — all `[required]`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`, `unbound required checks: none`, `not scheduled: a11y, browser, capability-conformance, release-evidence, responsive`, exit 0. Every required check resolved. |
| 5.2 `forge gate --timeout-secs 1800` | pass | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-23T12:07:16Z"` on revision `4689c7a23740` (the parent commit — the implementation is uncommitted while the Gate runs), with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. |
| 5.3 strict validation | pass | `openspec validate --all --strict --no-interactive` → `Totals: 151 passed, 0 failed (151 items)`. |
| 5.4 the change names | pass | `node scripts/check-openspec-change-names.mjs` → `PASS`. |
| 5.5 `git diff --check` | pass | `git diff --check` → no finding. |

### Judgement calls

- **The verdict lives in `src/docs/model.rs,** beside `ReviewStatus`,
  the vocabulary the outcome already uses. The report and the release
  lane read the same field, and the release lane compares against
  `crate::docs::ReviewStatus::Ok.label()` rather than a literal.
- **A needs-review run does not fail.** The doctor already judges a
  needs-review derivative as a warning; the command follows the same
  rule, so the exit code stays 0 and the journal state stays `done`.
- **`healthy` keeps its name and its type.** It is true only for
  `complete`, because the report's own doc comment called it the
  verdict.
- **The release lane's stage status is unchanged.** The derivative was
  written, so the locale is `delivered`; only the note changes. The
  stage's aggregate note names the locales that need review.
- **The journal detail gains the verdict,** because the state alone
  cannot say "complete" from "needs review".
- **The human renderer gains a `verdict:` line.** The summary already
  names the verdict in words; the line gives a reader the same word the
  JSON document publishes.
- **The existing `tests/docs_contract.rs` assertion changed.** It
  asserted `healthy == true` for a needs-review run — the defect
  itself. It now asserts `verdict == "needs-review"` and
  `healthy == false`.
