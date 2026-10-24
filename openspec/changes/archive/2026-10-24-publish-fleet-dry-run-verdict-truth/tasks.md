# Tasks: publish-fleet-dry-run-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the publish model: `src/publish/model.rs` (`PublishVerdict`, `verdict_for_report`).
- [x] 1.3 Read the fleet lane: `src/cli/fleet.rs` (the provider branch and the publish-pipeline branch), `src/cli/fleet_exec.rs` (`replay_publish_journal`).
- [x] 1.4 Read the provider lane: `src/cli/publish.rs` (`cmd_publish_provider`) and `src/publish/providers/engine.rs` (`compose_project_name`).
- [x] 1.5 Read the canonical capability: `openspec/specs/publish-dry-run-verdict-truth/spec.md`.
- [x] 1.6 Reproduce the defect on the current tree and record the fleet summary and the journal row.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the fleet provider lane reads the publish model's one rule.
- [x] 2.2 Decide where the health comes from: the provider's answer alone.
- [x] 2.3 Decide the options for the lane's row and reject the ones that keep a false claim, add a second vocabulary or invent a container.
- [x] 2.4 Decide the untouched surfaces: the publish pipeline, the fleet exec lane, `forge publish provider`, the provider contract, the publish contract version.

## 3. Implement

- [x] 3.1 `src/cli/fleet.rs`: the provider lane's health comes from the provider's answer; the verdict comes from the one rule; the row names the run; no container identity is composed.
- [x] 3.2 `tests/publish_fleet_dry_run_verdict_truth_contract.rs`: the CLI contract for a planned provider publish, a delivered one and a failed one.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A planned fleet provider publish records `planned`, is not healthy, names the verdict and records no container identity.
- [x] 4.4 A delivered fleet provider publish still records `done` and is healthy.
- [x] 4.5 A fleet provider publish that is not healthy still records `failed` and is counted as a failure.
- [x] 4.6 The publish pipeline lane, the fleet exec lane, `forge publish provider`, the provider contract and the publish contract version are unchanged.
- [x] 4.7 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.8 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.9 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 2400` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

- [x] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [x] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capability's purpose.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

All commands were run from the repository root with
`source /tmp/forge-env.sh` (so `CARGO_TARGET_DIR=/home/paul/code/forge/target`).

| Check | Command | Observed |
|---|---|---|
| Build | `cargo build --workspace` | `Finished \`dev\` profile`, exit 0 |
| Publish unit tests | `cargo test -p forge --lib publish::` | 193 passed; 0 failed |
| New contract test | `cargo test --test publish_fleet_dry_run_verdict_truth_contract` | 3 passed; 0 failed |
| Publish and fleet contracts | `cargo test --test publish_contract --test publish_dry_run_verdict_truth_contract --test fleet_contract --test fleet_cross_surface --test inventory_contract --test publish_observability_contract` | 11 + 4 + 16 + 7 + 9 + 8 passed; 0 failed |
| Full suite | `bash scripts/check-tests.sh` | exit 0; 237 `test result:` lines; 4246 passed; 0 failed; 1 ignored |
| Repository Gate | `forge gate --timeout-secs 2400` | exit 0; 8/8; `observed_at` 2026-10-24T14:07:06Z; revision 06eaa9ccd36f1ba055b89438c4b623f76c5c8d6d |
| Lint | `cargo clippy --workspace --all-targets --all-features` | exit 0; 39 warning lines (the pre-change baseline); no warning names a touched file |
| Format | `rustfmt --edition 2021 --check <file>` on each touched file | clean |
| File size | `wc -l` on each touched file | `src/cli/fleet.rs` 866, `tests/publish_fleet_dry_run_verdict_truth_contract.rs` 372 |
| Whitespace | `git diff --check` | no finding |
| Spec validation | `node .../openspec.js validate --all --strict --no-interactive` | `Totals: 161 passed, 0 failed (161 items)` |
| Change names | `node scripts/check-openspec-change-names.mjs` | `PASS` |

The contract test fails on the tree before the change: with
`src/cli/fleet.rs` reverted, all three cases fail — the planned
publish's summary reads `healthy: true` and carries no `verdict` field,
and the delivered and failed cases carry no `verdict` field either.
