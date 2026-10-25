# Tasks: api-agent-transition-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the shared rule: `src/agent/model.rs` (`AgentTransitionOutcome::journal_state()`).
- [x] 1.3 Read the API route: `src/api/handlers_changes.rs` (`handle_agent_transition`).
- [x] 1.4 Read the route's authorization and dispatch: `src/api/handlers_core.rs` (`authorize`), `src/api/router.rs`.
- [x] 1.5 Read the canonical capability: `openspec/specs/agent-transition-verdict-truth/spec.md`.
- [x] 1.6 Read the existing tests: `tests/agent_transition_verdict_truth_contract.rs`, `tests/web_agent_identity_readonly_contract.rs`, `tests/router_table_driven_contract.rs`.
- [x] 1.7 Reproduce the defect on the current tree and record both rows.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: every surface that records an agent transition reads the one rule.
- [x] 2.2 Decide where the rule lives: it already lives in `src/agent/model.rs`; the route joins it.
- [x] 2.3 Decide the options and reject the ones that keep a second decision in the route or that add a second capability for an existing rule.
- [x] 2.4 Decide the untouched surfaces: the route's response, the operation row, the session record, the command and the MCP tool.

## 3. Implement

- [x] 3.1 `src/api/handlers_changes.rs`: the route reads `outcome.journal_state()`.
- [x] 3.2 `openspec/specs/agent-transition-verdict-truth/spec.md`: the requirement names every surface (promoted by the archive).
- [x] 3.3 `tests/api_agent_transition_verdict_truth_contract.rs`: the route's row, the operation row, and the route's agreement with the command.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 The route journals the state the runtime reported.
- [x] 4.4 The route and the command record the same state.
- [x] 4.5 The route's response, the operation row and the session record are unchanged.
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

- [x] 6.1 Archive without `--skip-specs`, so the modified capability is promoted.
- [x] 6.2 Update the modified capability's purpose and strip the archive's trailing blank lines by hand.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

### The reproduced defect

The same refused transition, driven through the command and through the
HTTP route against one registry:

```
# the command
$ forge --registry /tmp/api-agent/registry.db agent start /tmp/api-agent/proj \
    --session s1 --provider codex
# the row: ('agent', 'api-agent', 'unsupported', '... start -> `unsupported`')

# the HTTP route
$ POST /v1/projects/api-agent/agents  {"session":"s1","transition":"start","provider":"codex"}
202 Accepted
{"agent":{"transition":{"state":"unsupported", ...}}, ...}
# the row: ('api', 'api-agent', 'done', 'api agent `api-agent` session `s1` start -> `unsupported`')
```

The route's row said `done` for a transition the runtime refused, and
its own detail said `-> unsupported` in the same row: the row
contradicted itself.

The state was a literal in the route:

```rust
// src/api/handlers_changes.rs, before
let _ = registry.record_operation("api", id, "done", &detail);
```

### The change

| Surface | Before | After |
|---|---|---|
| `src/api/handlers_changes.rs` transition row | `"done"` | `&outcome.journal_state()` |
| `openspec/specs/agent-transition-verdict-truth/spec.md` | "The command and the MCP tool SHALL read the same rule." | "Every surface that records an agent transition — the command, the MCP tool and the HTTP route — SHALL read the same rule." |

The rows after the change:

```
('agent', 'api-agent', 'unsupported', '... start -> `unsupported`')
('api', 'api-agent', 'unsupported', 'api agent `api-agent` session `s1` start -> `unsupported`')
('api.agent', 'api-agent', 'done', 'api.agent completed')
```

The third row is the request's lifecycle, not the transition's verdict:
the route's own lifecycle helper reserves it `pending` and finalizes it
`done` when the handler returns. The route's body validation runs
before the helper reserves the operation, so a request the route
refuses (`400`) writes no operation row at all.

### The checks

| Check | Result | Evidence |
|---|---|---|
| The reproduced defect | reproduced before the change | The route's row above: `('api', 'api-agent', 'done', '... start -> `unsupported`')`. The state was the literal `"done"` in `src/api/handlers_changes.rs`. |
| The contract test fails without the change | PASS | The new test was run against the unfixed route first: `the_http_route_journals_the_state_the_runtime_reported` failed with `left: "done", right: "unsupported"`, and `the_route_and_the_command_agree_on_the_state_they_record` failed with `left: "unsupported", right: "done"` — the two surfaces disagreed. With the fix, all three cases pass. |
| The route journals the state the runtime reported | PASS | `the_http_route_journals_the_state_the_runtime_reported`: the route's row is `unsupported`, never `done`, and its detail still names `-> `unsupported``. |
| The route and the command agree | PASS | `the_route_and_the_command_agree_on_the_state_they_record`: the command's row and the route's row for the same refused transition carry the same state. |
| The two rows are two facts | PASS | `the_route_writes_a_transition_row_and_an_operation_row`: the transition's row is `unsupported` and names the transition; the operation's row is `done` and says `api.agent completed`. |
| One rule, no second decision | PASS | `src/api/handlers_changes.rs` calls `outcome.journal_state()`; the file contains no other state literal for a transition row, and `src/agent/model.rs` is unchanged. |
| The untouched surfaces | PASS | `git diff` changes one line of `src/api/handlers_changes.rs` and touches no other source file. The route's `202` envelope, the transition body, the session record and the operation row are unchanged. The existing agent suites pass unchanged: `agent_contract` 11 passed, `agent_transition_verdict_truth_contract` 4 passed, `supervised_agent_cross_surface` 8 passed, `api_cross_surface` 6 passed, `router_table_driven_contract` 6 passed, `web_agent_identity_readonly_contract` 10 passed. |
| Repository test suite | PASS | `bash scripts/check-tests.sh` — the repository Gate's own `tests` command, and the same suite CI's `test` job runs — exited 0; the 246 `test result:` lines sum to **4287 passed; 0 failed; 1 ignored**. The pre-change baseline was 245 lines and 4284 passed, so this change adds exactly the 3 tests it declares and one new test binary. |
| Repository Gate | PASS, 8/8 | The owning agent's run on the final tree: exit 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-25T11:06:08Z"` on revision `a934708e873458073489d08bf83356e291b4c204` (the parent commit — the implementation is uncommitted while the Gate runs), with all eight required checks — `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` — in state `pass`. The rehearsal before it (`forge gate --dry-run`) resolved the same 8 required checks, `unbound required checks: none`, `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within`. The advisory database's fetch was resolved to the local mirror of the same cached database (`/home/paul/.cache/advisory-db-mirror.git`, commit `640bf5a9`) through `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_0=url.file:///home/paul/.cache/advisory-db-mirror.git.insteadOf` / `GIT_CONFIG_VALUE_0=https://github.com/RustSec/advisory-db.git`, and `CARGO_HTTP_PROXY=http://127.0.0.1:1` made the unreachable crates.io index update fail fast instead of waiting for the TCP timeout. Both settings applied to that one Gate process only. |
| OpenSpec, change names, whitespace | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 168 passed, 0 failed (168 items)`; `node scripts/check-openspec-change-names.mjs` → `PASS`; `git diff --check` and `git diff --cached --check` → no finding. Archived without `--skip-specs`; the `agent-transition-verdict-truth` requirement was modified with all four canonical scenario names kept and two scenarios added, its purpose now names every surface, and the archive's trailing blank lines were stripped from the canonical spec by hand. `openspec list` reports no active changes. |
| Lint and formatting | PASS | `cargo clippy --workspace --all-targets --all-features` exited 0 with 39 lines starting with `warning`, the same count as the pre-change baseline, and no warning names a touched file. Two `needless_borrow` warnings the new test first carried were fixed before the archive. `rustfmt --edition 2021 --check` is clean on both touched Rust files, and `cargo fmt --all` was never run. No pre-existing rustfmt-dirty file was touched. Every touched file stays under the 1000 physical line limit: `src/api/handlers_changes.rs` 668, `tests/api_agent_transition_verdict_truth_contract.rs` 252. |
| The worker | the owning agent wrote the change | The worker subagents in this programme have been unreliable, so the owning agent wrote this change. An `explore` subagent found the candidate and the owning agent verified it in the code, reproduced it with the new contract test, and wrote the fix. |
