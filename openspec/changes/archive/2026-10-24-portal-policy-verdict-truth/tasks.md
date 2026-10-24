# Tasks: portal-policy-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the portal: `src/portal/sections.rs`, `src/portal/model.rs`, `src/portal/portal_tests.rs`.
- [x] 1.3 Read the doctor: `src/doctor/probes.rs` (`detect_driftwatch`), `src/doctor/assessment.rs` (`PolicyVerdict`), `src/doctor/delivery.rs` (the house rule style).
- [x] 1.4 Read the registry: `src/registry/model.rs` (the `projects` table and its write paths).
- [x] 1.5 Read the canonical capability: `openspec/specs/control-plane-portal/spec.md`.
- [x] 1.6 Reproduce the defect on the current tree and record the rendered Policies section.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the portal states a status only from an observation it made.
- [x] 2.2 Decide where the rule lives: in the portal's section builder, read by the Policies section.
- [x] 2.3 Decide the options for the Policies section's status and reject the ones that keep a false claim, add a side effect to a read-only plane or break the section contract.
- [x] 2.4 Decide the untouched surfaces: the doctor, the other eleven sections, the projects section's liveness signal, the registry schema, the contract version.

## 3. Implement

- [x] 3.1 `src/doctor/probes.rs`: the DriftWatch artifact detector becomes `pub(crate)`.
- [x] 3.2 `src/portal/sections.rs`: one rule judges a project's policies; the entry carries the observed artifacts.
- [x] 3.3 `tests/portal_policy_verdict_truth_contract.rs`: the CLI contract for a project with no recorded verdict, with a recorded verdict and with an unreadable directory.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A project with no recorded policy verdict reports `unknown` and names what the portal looked at.
- [x] 4.4 A project with a recorded policy verdict reports that verdict.
- [x] 4.5 A project whose directory cannot be read reports `unavailable`.
- [x] 4.6 The other eleven sections, the projects section, the dashboard's rollup, the doctor's findings and the registry are unchanged.
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
| Portal unit tests | `cargo test -p forge --lib portal::` | 34 passed; 0 failed |
| New contract test | `cargo test --test portal_policy_verdict_truth_contract` | 3 passed; 0 failed |
| Full suite | `bash scripts/check-tests.sh` | exit 0; 238 `test result:` lines; 4253 passed; 0 failed; 1 ignored |
| Repository Gate | `forge gate --timeout-secs 2400` | exit 0; 8/8; `observed_at` 2026-10-24T14:47:05Z; revision 585dec3601c7c8e15a1bdb984df823ba21253d28 |
| Lint | `cargo clippy --workspace --all-targets --all-features` | exit 0; 39 warning lines (the pre-change baseline); no warning names a touched file |
| Format | `rustfmt --edition 2021 --check <file>` on each touched file | clean |
| File size | `wc -l` on each touched file | `src/portal/sections.rs` 880, `src/portal/portal_tests.rs` 957, `src/doctor/probes.rs` 936, `tests/portal_policy_verdict_truth_contract.rs` 172 |
| Whitespace | `git diff --check` | no finding |
| Spec validation | `node .../openspec.js validate --all --strict --no-interactive` | `Totals: 162 passed, 0 failed (162 items)` |
| Change names | `node scripts/check-openspec-change-names.mjs` | `PASS` |

The contract test fails on the tree before the change: with
`src/portal/sections.rs` reverted, all three cases fail — the entries
read `status: "ok"`.
