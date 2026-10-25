# Tasks: agent-transition-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the writers: `src/cli/agent.rs` (the run-spec and the transition writers), `src/mcp/methods.rs` (the same two).
- [x] 1.3 Read the outcome: `src/agent/model.rs` (`AgentTransitionOutcome`, `SessionState`), `src/agent/sessions.rs` (`run_spec`), `src/agent/transitions.rs` (`apply_transition`).
- [x] 1.4 Read the canonical capability: `openspec/specs/agent-runtime-workflows/spec.md`.
- [x] 1.5 Read the existing tests: `tests/agent_contract.rs`, `tests/supervised_agent_cross_surface.rs`.
- [x] 1.6 Reproduce the defect on the current tree and record the journal rows it wrote.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: the journal state is the outcome the runtime reported — the run-spec verdict when the supervisor reported one, the session state otherwise.
- [x] 2.2 Decide where the rule lives: on the outcome, in the agent module, read by the CLI and the MCP tool.
- [x] 2.3 Decide the options and reject the ones that keep a completed action on the record, invent a second vocabulary or refuse an operation the canonical requirement says must stay answerable.
- [x] 2.4 Decide the untouched surfaces: the session record, the supervisor path, the command's envelope, the other lanes.

## 3. Implement

- [x] 3.1 `src/agent/model.rs`: `AgentTransitionOutcome::journal_state()` — the one rule.
- [x] 3.2 `src/cli/agent.rs`: both writers read the rule; the run-spec detail names the verdict it saw or that it saw none.
- [x] 3.3 `src/mcp/methods.rs`: both writers read the rule; the run-spec detail names the verdict it saw or that it saw none.
- [x] 3.4 `tests/agent_transition_verdict_truth_contract.rs`: the refused transition, the refused run-spec and the supervised run.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A refused transition journals the reported state, never `done`.
- [x] 4.4 A run-spec with no reported verdict journals the reported state and says it saw none.
- [x] 4.5 A supervised verified run still journals `done`.
- [x] 4.6 The session record, the supervisor path, the command's envelope and the other lanes are unchanged.
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
| New contract test | `cargo test --test agent_transition_verdict_truth_contract` | 4 passed; 0 failed |
| Existing agent tests | `cargo test --test agent_contract --test supervised_agent_cross_surface` | 11 passed and 8 passed; 0 failed |
| Full suite | `bash scripts/check-tests.sh` | exit 0 on the re-run; 242 `test result:` lines; 4274 passed; 0 failed; 1 ignored |
| Repository Gate | `forge gate --timeout-secs 2400` | exit 0; 8/8; `observed_at` 2026-10-25T05:55:11Z; revision d29038ad67b2a19ef3efbb928772d91666853c06 |
| Lint | `cargo clippy --workspace --all-targets --all-features` | exit 0; 39 warning lines (the pre-change baseline); no warning names a touched file |
| Format | `rustfmt --edition 2021 --check <file>` on each touched file | clean |
| File size | `wc -l` on each touched file | `src/agent/model.rs` 267, `src/cli/agent.rs` 389, `src/mcp/methods.rs` 896, `tests/agent_transition_verdict_truth_contract.rs` 329 |
| Whitespace | `git diff --check` | no finding |
| Spec validation | `node .../openspec.js validate --all --strict --no-interactive` | `Totals: 166 passed, 0 failed (166 items)` |
| Change names | `node scripts/check-openspec-change-names.mjs` | `PASS` |

The contract test fails on the tree before the change: with the three
touched source files reverted, three of the four cases fail — the
refused transition, the run-spec with no verdict and the MCP tool all
read `done` while their own detail says `unsupported`. The fourth case
(the verified supervisor run) passes on both trees.

One pre-existing flake was observed and is not this change's: the first
full-suite run failed on `tests/mcp_contract.rs::mcp_repeated_isolated_round_trip_is_stable`,
which compares two round trips of the project catalog. The two
documents differed only in the catalog's `observed_at` timestamp
(`2026-10-25T05:30:39Z` against `2026-10-25T05:30:40Z`): the two runs
straddled a second boundary. The test passes on re-run (25 consecutive
runs of that test alone passed, and the full suite re-run exited 0).
It is the same class as the recorded `tests/checker_contract.rs`
`generated_at` flake. The remediation path, when that test is next
touched: compare the two results with the observation timestamp
removed, because the timestamp is the time of the observation, not
state.

The defect, reproduced on the tree before the change:

```
$ forge --registry /tmp/agent-probe/registry.db agent run-spec /tmp/agent-probe --session s1
  "state": "unsupported",
  "registry_detail": "agent `agent-probe` run_spec session `s1` -> `unsupported` (verdict `done`)"
$ sqlite3 /tmp/agent-probe/registry.db "SELECT kind, state, detail FROM operations"
('agent', 'agent-probe', 'done', 'agent `agent-probe` run_spec session `s1` -> `unsupported` (verdict `done`)')
('agent', 'agent-probe', 'done', 'agent `agent-probe` session `s1` provider `codex` spec `` -> `unsupported`')
```

The same commands on the tree after the change:

```
('agent', 'agent-probe', 'unsupported', 'agent `agent-probe` run_spec session `s1` -> `unsupported` (verdict `none reported`)')
('agent', 'agent-probe', 'unsupported', 'agent `agent-probe` session `s1` provider `codex` spec `` -> `unsupported`')
```
