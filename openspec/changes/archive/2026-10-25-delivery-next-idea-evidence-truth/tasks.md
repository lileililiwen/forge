# Tasks: delivery-next-idea-evidence-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the route: `src/api/lifecycle_exec/delivery.rs`, and its siblings in `src/api/lifecycle_exec/`.
- [x] 1.3 Read the delivery lane: `src/api/delivery.rs` (the publish and the reconcile route).
- [x] 1.4 Read the canonical capability: `openspec/specs/web-lifecycle-execution/spec.md`.
- [x] 1.5 Read the existing test: `tests/web_lifecycle_execution_contract.rs` (the next-idea case).
- [x] 1.6 Reproduce the defect on the current tree and record the journal row it wrote.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the row states the transition the server recorded and never claims an unobserved publish.
- [x] 2.2 Decide where the caller's note belongs: in the row, marked as the caller's report.
- [x] 2.3 Decide the options and reject the ones that keep the invented claim, put a second copy of the publish vocabulary in the lifecycle lane, or guess a refusal.
- [x] 2.4 Decide the untouched surfaces: the publish lanes, the response, the refusals, the note's contract, the frontend, the other lifecycle routes.

## 3. Implement

- [x] 3.1 `src/api/lifecycle_exec/delivery.rs`: the detail states the transition and marks the caller's report.
- [x] 3.2 `tests/web_lifecycle_execution_contract.rs`: the detail assertions follow the new rule.
- [x] 3.3 `tests/delivery_next_idea_evidence_truth_contract.rs`: the row with no note, the row with a note, and the unchanged refusals.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A confirmed request with no note writes a row that claims no publish.
- [x] 4.4 A confirmed request with a note keeps the note marked as the caller's report.
- [x] 4.5 The unconfirmed, over-long and credential-shaped refusals are unchanged and journal nothing.
- [x] 4.6 The publish lanes, the delivery publish lane, the response, the other lifecycle routes and the frontend are unchanged.
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
| New contract test | `cargo test --test delivery_next_idea_evidence_truth_contract` | 3 passed; 0 failed |
| Lifecycle contract test | `cargo test --test web_lifecycle_execution_contract` | 9 passed; 0 failed |
| Full suite | `bash scripts/check-tests.sh` | exit 0; 241 `test result:` lines; 4270 passed; 0 failed; 1 ignored |
| Repository Gate | `forge gate --timeout-secs 2400` | exit 0; 8/8; `observed_at` 2026-10-25T04:26:52Z; revision 17e7644fe3bd8b2ce225cd089311ed7420674291 |
| Lint | `cargo clippy --workspace --all-targets --all-features` | exit 0; 39 warning lines (the pre-change baseline); no warning names a touched file |
| Format | `rustfmt --edition 2021 --check <file>` on each touched file | clean |
| File size | `wc -l` on each touched file | `src/api/lifecycle_exec/delivery.rs` 109, `tests/delivery_next_idea_evidence_truth_contract.rs` 233, `tests/web_lifecycle_execution_contract.rs` 565 |
| Whitespace | `git diff --check` | no finding |
| Spec validation | `node .../openspec.js validate --all --strict --no-interactive` | `Totals: 165 passed, 0 failed (165 items)` |
| Change names | `node scripts/check-openspec-change-names.mjs` | `PASS` |

The contract test fails on the tree before the change: with the route
reverted, two of the three cases fail — the row without a note reads
`delivery.next-idea: publish landed; next loop proposed`, and the row
with a note reads `delivery.next-idea: publish landed`, with the
caller's report stated as the server's own finding. The third case
(the unchanged refusals) passes on both trees.

The defect, reproduced on the tree before the change:

```
$ POST /v1/admin/projects/p1/delivery/next-idea '{"confirm":true}'
200 OK
$ sqlite3 registry.db "SELECT kind, state, detail FROM operations WHERE kind='delivery.next-idea'"
delivery.next-idea|done|delivery.next-idea: publish landed; next loop proposed
```

The same request on the tree after the change:

```
delivery.next-idea|done|delivery.next-idea: transition=proposed
```
