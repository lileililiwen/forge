# Tasks: release-dry-run-evidence-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the release engine: `src/release/engine/apply.rs` (`apply_release`, `run_stage`, `run_commit_stage`, `run_tag_stage`, the note).
- [x] 1.3 Read the stages: `src/release/engine/stages.rs` (push, mirror, package, container, docs, notes).
- [x] 1.4 Read the request and the report: `src/release/model.rs` (`ReleaseRequest`, `ReleaseReport`).
- [x] 1.5 Read the transport: `src/cli/release.rs` (`cmd_release_apply`, the journal row).
- [x] 1.6 Read the canonical capability: `openspec/specs/release-publishing/spec.md` ("Resumable multi-destination publication").
- [x] 1.7 Reproduce the defect: drive `apply_release` with `confirm: false, dry_run: true` on the release fixture and read the report and the repository.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: a stage writes only when the release is confirmed and the run is not a dry run.
- [x] 2.2 Decide where the rule lives: on `ReleaseRequest`, so every stage reads one decision.
- [x] 2.3 Decide the refusal note for each cause, and that a plan-only stage keeps its plan evidence.
- [x] 2.4 Decide the untouched surfaces: the stage status vocabulary, the report's fields, the contract version, the state file, the adapters.
- [x] 2.5 Decide the journal state for a dry run.

## 3. Implement

- [x] 3.1 `src/release/model.rs`: `ReleaseRequest::may_write()` and `ReleaseRequest::write_refusal()`.
- [x] 3.2 `src/release/engine/apply.rs`: the commit stage and the tag stage consult the gate; the dry-run note names the planned stages.
- [x] 3.3 `src/release/engine/stages.rs`: the push, mirror, package, container, docs and notes stages consult the gate; the plan-only mirror stage is not a delivery.
- [x] 3.4 `src/cli/release.rs`: a dry run is recorded in the journal as `planned`.
- [x] 3.5 The engine tests in `src/release/engine/engine_tests.rs`.
- [x] 3.6 `tests/release_dry_run_truth_contract.rs`: the CLI contract for a dry run.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A dry run creates no tag and no commit.
- [x] 4.4 A dry run's stage outcomes are never a delivery.
- [x] 4.5 A dry run's note says that no side effect was attempted.
- [x] 4.6 A confirmed, non-dry release still writes: the tag exists and the stage is `delivered`.
- [x] 4.7 A plan-only mirror stage is not a delivery and keeps its plan evidence.
- [x] 4.8 The release report's fields, the stage status vocabulary and the contract version are unchanged.
- [x] 4.9 The release state file is still written.
- [x] 4.10 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.11 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.12 Every touched file stays under the 1000 physical line limit.

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
| 1.7 the reproduced defect | reproduced, before the change | The owning agent's own run of the release engine on the release fixture, before the change: `NOTE=dry-run: no side effects were attempted`; `STAGE=tag STATUS=delivered NOTE=tag v1.0.0 delivered for release rel-scratch-1.0.0-aef2a7e9d820`; `TAGS=v1.0.0`. The note says no side effect was attempted, a stage outcome in the same report says a tag was delivered, and the tag was in the repository. |
| 4.1 `cargo build` | pass | `cargo build --workspace` exited 0; `Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.50s`. |
| 4.2 the repository test suite | pass | `bash scripts/check-tests.sh` — the repository Gate's own `tests` command, and the same suite CI's `test` job runs — exited 0; the 229 `test result:` lines sum to **4201 passed; 0 failed; 1 ignored**. The pre-change baseline, measured with the same script, was 228 lines and 4194 passed, so this change adds exactly the 7 tests it declares (3 engine unit tests, 4 contract tests) and no new test binary. |
| 4.3 a dry run writes nothing | pass | `apply_dry_run_writes_no_tag_and_no_commit` (engine): a dry run leaves the fixture's tag list empty and `rev-parse HEAD` unchanged. `dry_run_apply_writes_no_tag_and_records_a_planned_journal_row` (CLI): the project's `git tag --list` is empty and `rev-parse HEAD` is the fixture's first commit. The fixture's changelog carries an unreleased entry, so the commit stage had something to commit if it were allowed to write. |
| 4.4 no stage claims a delivery | pass | Both dry-run tests assert `status != "delivered"` for every entry of `stage_outcomes`; the engine test asserts `outcome.status != STATUS_DELIVERED` for every outcome. |
| 4.5 the dry-run note | pass | `apply_dry_run_writes_no_tag_and_no_commit` asserts the note contains `no side effects were attempted`; the CLI test asserts the same over the JSON document. The note is now `dry-run: no side effects were attempted; N stage(s) planned`. |
| 4.6 a confirmed release still writes | pass | `apply_confirmed_release_still_creates_the_tag` (engine): the fixture's tag list is exactly `["v1.0.0"]` and the tag stage's status is `delivered`. `confirmed_apply_still_delivers_the_tag` (CLI): `git tag --list` is `v1.0.0` and the JSON report's tag stage status is `delivered`. |
| 4.7 the plan-only mirror stage | pass | `dry_run_reports_the_mirror_plan_without_claiming_a_delivery` (CLI): a project whose manifest declares `distribution: {primary: github, mirrors: [gitee]}` produces a mirror stage whose status is `disabled`, whose note contains `mirror plan only` and `no side effect was attempted`, and whose evidence still names the planned `github` primary and the `gitee` mirror. Before the change the same stage was recorded as `delivered`. |
| 4.8 the untouched contract | pass | `git diff` shows no change to `ReleaseReport`'s fields, to the stage status constants (`delivered`, `skipped`, `disabled`, `failed`, `conflict`, `diverged`) or to `RELEASE_CONTRACT_VERSION`. The diff touches only the gate, the per-stage refusal, the dry-run note and the journal state. |
| 4.9 the release state file | pass | `apply_persists_release_state_for_retry` still passes in `cargo test -p forge --lib release::engine::` (23 passed): the state path is a file. A `disabled` stage is never matched by the retry's `already_delivered` check, which only matches `delivered` and `skipped`, so a dry run cannot make a later apply skip a stage. |
| 4.10 rustfmt | pass | `rustfmt --edition 2021 --check` is clean on all six touched Rust files. `cargo fmt --all` was never run, and no pre-existing rustfmt-dirty file was touched. |
| 4.11 clippy | pass | `cargo clippy --workspace --all-targets --all-features` exited 0 with 39 warnings, the same count as the pre-change baseline, and no warning points at a touched file. |
| 4.12 the source file line counts | pass | Every touched file stays under the 1000 physical line limit: `src/release/model.rs` 547, `src/release/engine/apply.rs` 502, `src/release/engine/stages.rs` 686, `src/release/engine/engine_tests.rs` 997, `src/cli/release.rs` 638, `tests/release_dry_run_truth_contract.rs` 362. |
| 5.1 `forge gate --dry-run` | pass | `./target/debug/forge gate --dry-run` resolved the same 8 required checks — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests` — with `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within` and `unbound required checks: none`, exit 0. |
| 5.2 `forge gate --timeout-secs 1800` | pass | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-24T01:45:37Z"` on revision `5e3f8be25017f2f48fa20826bae82701557563f7` (the parent commit — the implementation is uncommitted while the Gate runs), with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. |
| 5.2 the two earlier Gate runs on the same tree | environmental, recorded | Two Gate runs on the same tree did not pass, and neither failure is attributable to this change. (a) At `2026-10-24T00:54:59Z` the `tests` check exited 101 with its captured output stopping mid-line, while another workspace (`/home/paul/code/alltools-platform/rust`) was running its own cargo build on the same 14 GB host; the identical command on the identical tree passes — `bash scripts/check-tests.sh` with the Gate's own environment exited 0 twice with 4201 passed / 0 failed. (b) At `2026-10-24T01:24:42Z` the `security` check went to `REVIEW_REQUIRED` because the `project-runtime` adapter exceeded its 60000 ms deadline while fetching the advisory database; `cargo audit` measured 1m56s on that run because the cached advisory database still had to be materialised from the local mirror, and 2.757s on the next run once it was current. The remedy is the one this repository already uses and which the earlier changes recorded: resolve the advisory database's fetch to the local mirror of the same cached database through `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_0=url.file:///home/paul/.cache/advisory-db-mirror.git.insteadOf` / `GIT_CONFIG_VALUE_0=https://github.com/RustSec/advisory-db.git`, and set `CARGO_HTTP_PROXY=http://127.0.0.1:1` so the unreachable crates.io index update fails fast instead of waiting for the TCP timeout. Both settings applied to that one Gate process only: no file in this repository and no global cargo or git configuration was changed. The change added no failure. |
| 5.3 strict validation | pass | `openspec validate --all --strict --no-interactive` → `Totals: 153 passed, 0 failed (153 items)`. |
| 5.4 the change names | pass | `node scripts/check-openspec-change-names.mjs` → `PASS`. |
| 5.5 `git diff --check` | pass | `git diff --check` → no finding. |

### Judgement calls

- **The dry run wins over a confirmation.** `forge release apply --confirm
  --dry-run` used to write, while the report said "dry-run: no side
  effects were attempted". A dry run is the stronger promise the report
  makes, so it now wins: the gate stays closed. The alternative —
  refusing the contradictory flag combination — would have been louder
  but would have turned a working invocation into an error.
- **The gate is checked first in every write-capable stage.** A stage
  that may not write records the refusal, so the reason a stage did
  not write is always the gate's reason and never a configuration
  detail. The configuration detail is still reported by `forge release
  prepare`, and by a confirmed run of the same stage.
- **The plan-only mirror stage keeps its evidence.** It still collects
  the per-remote plan; only the status changed, from `delivered` to
  `disabled`, and the note now names the refusal.
- **`healthy` is unchanged.** It still means "no stage failed". A dry
  run that planned every stage is healthy because nothing failed, and
  its note says that nothing was written. No new verdict type was
  added to the release report.
- **`forge release prepare` is unchanged.** It records `done`/`blocked`
  because the preparation itself completed; the dry-run claim is the
  apply's claim.
- **The worker's session was lost before it reported.** The worker wrote
  the implementation and its tests into the worktree and then lost its
  session during the full-suite run. The owning agent reviewed the
  whole diff, re-ran every check on the final tree, added the fourth
  contract case (the plan-only mirror stage) that the worker had not
  written, and produced the evidence above.
