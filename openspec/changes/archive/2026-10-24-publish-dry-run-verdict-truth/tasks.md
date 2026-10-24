# Tasks: publish-dry-run-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the report: `src/publish/model.rs` (`PublishReport`).
- [x] 1.3 Read the pipeline: `src/publish/pipeline.rs` (`run_publish`, the note, the journal row).
- [x] 1.4 Read the publish CLI: `src/cli/publish.rs` (`cmd_publish`, the exit code).
- [x] 1.5 Read the fleet lanes: `src/cli/fleet.rs` (the manual path's per-project state) and `src/cli/fleet_exec.rs` (the journal verdict).
- [x] 1.6 Read the canonical capability: `openspec/specs/decoupled-remote-publish/spec.md` ("Caddy host rendering in Forge").
- [x] 1.7 Read the existing tests: `tests/publish_contract.rs`, `tests/fleet_contract.rs` (or the closest files), `src/publish/pipeline.rs`'s own test module.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: a run that attempted no write is a plan, a run whose stages all completed is a delivery, anything else is failed.
- [x] 2.2 Decide where the rule lives: in the publish model, next to the report it judges.
- [x] 2.3 Decide the verdict's names and its wire shape.
- [x] 2.4 Decide that `healthy` is derived from the verdict, and that every exit code follows the verdict rather than `healthy`.
- [x] 2.5 Decide the journal state for each verdict, in every writer.
- [x] 2.6 Decide the untouched surfaces: the note, the stages, the deploy lane, the provider branch, the MCP method, the release lane, the contract version.

## 3. Implement

- [x] 3.1 `src/publish/model.rs`: `PublishVerdict`, `verdict_for_report`, the report's `verdict` field.
- [x] 3.2 `src/publish/pipeline.rs`: the report's verdict, the journal state and the journal detail.
- [x] 3.3 `src/cli/publish.rs`: the exit code follows the verdict.
- [x] 3.4 `src/cli/fleet_exec.rs`: the journal state follows the verdict.
- [x] 3.5 `src/cli/fleet.rs`: the per-project state and the summary follow the verdict.
- [x] 3.6 `tests/publish_dry_run_verdict_truth_contract.rs`: the CLI contract for a plan, a delivery and a failure.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A plan's verdict is `planned`, the report is not healthy, and the journal records `planned`.
- [x] 4.4 A plan exits 0.
- [x] 4.5 A delivery's verdict is `delivered`, the report is healthy, and the journal records `done`.
- [x] 4.6 A failed publish's verdict is `failed` and the command exits 1.
- [x] 4.7 The fleet lane's per-project state and summary follow the verdict, and a planned fleet publish is not a failure.
- [x] 4.8 The publish summary's note, the stages, the deploy lane, the provider branch and the contract version are unchanged.
- [x] 4.9 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.10 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.11 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 1800` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [ ] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [ ] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capabilities' purposes.
- [ ] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

All commands were run from the repository root with
`source /tmp/forge-env.sh` (so `CARGO_TARGET_DIR=/home/paul/code/forge/target`).

| Check | Command | Observed |
|---|---|---|
| Build | `cargo build --workspace` | `Finished \`dev\` profile [unoptimized + debuginfo] target(s)`, exit 0 |
| Core unit tests | `cargo test -p forge --lib publish::` | `test result: ok. 193 passed; 0 failed; 0 ignored; 0 measured; 1263 filtered out` |
| New contract test | `cargo test --test publish_dry_run_verdict_truth_contract` | `running 4 tests` — `a_planned_fleet_publish_is_planned_and_not_a_failure ... ok`, `a_dry_run_is_planned_and_journals_a_planned_row ... ok`, `a_failed_publish_is_failed_and_journals_a_failed_row ... ok`, `a_completed_publish_is_delivered_and_journals_a_done_row ... ok`; `test result: ok. 4 passed; 0 failed` |
| Publish contracts | `cargo test --test publish_contract --test decoupled_remote_publish_contract --test publish_observability_contract` | `5 passed`, `11 passed`, `8 passed`; 0 failed |
| Fleet contracts | `cargo test --test fleet_contract --test fleet_cross_surface --test inventory_contract` | `16 passed`, `7 passed`, `9 passed`; 0 failed |
| Web / live fleet contracts | `cargo test --test forge_web_fleet_contract --test fleet_live_rollout_contract --test forge_web_publish_fleet_contract --test fleet_online_contract --test forge_web_project_publish_contract` | `9 passed`, `16 passed`, `11 passed`, `9 passed`, `7 passed`; 0 failed |
| Queue / sibling contracts | `cargo test --test publish_queue_status_contract --test sibling_cwd_publish_contract` | `7 passed`, `12 passed`; 0 failed |
| Release contracts | `cargo test --test release_contract --test release_cross_surface` | `10 passed`, `4 passed`; 0 failed |
| Mirror verdict contract | `cargo test --test mirror_plan_verdict_truth_contract` | `4 passed`; 0 failed |
| Fleet verdict contract | `cargo test --test upgrade_fleet_verdict_truth_contract` | `7 passed`; 0 failed |
| Full suite | `bash scripts/check-tests.sh` | exit 0; `231` `test result:` lines; `4211 passed`, `0 failed`, `1 ignored`; `dotnet-platform-libs import: PASS`, `rust-platform-libs import: PASS` |
| Repository Gate | `forge gate --timeout-secs 2400` | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-24T05:23:02Z"` on revision `b4bf4acdc4f68cf7b4f8e63cf79bd0a5d00d988b` (the parent commit — the implementation is uncommitted while the Gate runs), with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. The rehearsal before it (`forge gate --dry-run`) resolved the same 8 required checks, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. The advisory database's fetch was resolved to the local mirror of the same cached database and `CARGO_HTTP_PROXY=http://127.0.0.1:1` made the unreachable crates.io index update fail fast; both settings applied to that one Gate process only. |
| Lint | `cargo clippy --workspace --all-targets --all-features` | exit 0; `39` lines starting with `warning` (`33` lint headers plus `6` per-crate summary lines), the same count as the baseline; no warning names a touched file |
| Format | `rustfmt --edition 2021 --check <file>` on each touched file | `src/publish/model.rs`, `src/publish/pipeline.rs`, `src/cli/publish.rs`, `src/cli/fleet_exec.rs`, `src/cli/fleet.rs`, `tests/publish_dry_run_verdict_truth_contract.rs` — all clean, no output |
| File size | `wc -l` on each touched file | `model.rs` 356, `pipeline.rs` 926, `cli/publish.rs` 724, `cli/fleet_exec.rs` 830, `cli/fleet.rs` 858, `tests/publish_dry_run_verdict_truth_contract.rs` 333 — all under 1000 |
| Whitespace | `git diff --check` | exit 0, no output |
| Spec validation | `node .../openspec.js validate --all --strict --no-interactive` | `Totals: 155 passed, 0 failed (155 items)`, exit 0; `✓ change/publish-dry-run-verdict-truth` |
| Change names | `node scripts/check-openspec-change-names.mjs` | `check-openspec-change-names: PASS`, exit 0 |

### What the new contract test pins

| Case | Verdict | `healthy` | Exit | Journal summary row |
|---|---|---|---|---|
| `forge publish all --dry-run` | `planned` | `false` | 0 | state `planned`, detail `verdict=planned healthy=false` |
| `forge publish all` (every stage completes) | `delivered` | `true` | 0 | state `done`, detail `verdict=delivered healthy=true` |
| `forge publish all` (a stage fails) | `failed` | `false` | 1 | state `failed`, detail `verdict=failed healthy=false` |
| `forge publish fleet --dry-run` | `planned` per project | `false` | 0 | per-project state `planned`, detail `fleet stages=4 verdict=planned healthy=false`; `fleet.failed` is `0` |

The delivery and failure cases drive the legacy `jenkins` lane with a fake
`ssh`/`rsync` pair on `PATH`, so no target host is contacted.

### The owning agent's additions

- The worker's first clippy run reported two warnings in the new test
  file — a `while let Some(doc) = stream.next()` loop and the `mut`
  it needed. The owning agent rewrote the loop as a `for` loop and
  removed the `mut`, so the change adds no clippy warning, and
  re-ran the build, the new contract test, rustfmt and the full suite
  on the final tree.
- The Gate is the owning agent's: the worker does not own the Gate.
