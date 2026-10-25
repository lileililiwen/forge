# Tasks: resolve-plan-verdict-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`, `.ai-rules/architecture.md`.
- [x] 1.2 Read the resolver lanes: `src/cli/component.rs` (the `Resolve` arm), `src/cli/ui_pattern.rs` (the `Resolve` arm).
- [x] 1.3 Read the core: `src/component/engine.rs`, `src/ui_pattern/resolve.rs`, `src/core/mod.rs`, `src/registry/model.rs` (`record_operation`).
- [x] 1.4 Read the canonical capabilities: `openspec/specs/semantic-component-registry/spec.md`, `openspec/specs/semantic-ui-patterns/spec.md`.
- [x] 1.5 Read the precedent: `src/release/model.rs` (`ReleaseRun`), `src/deploy/model.rs` (`DeployVerdict`).
- [x] 1.6 Reproduce the defect on the current tree and record the journal rows both lanes wrote.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: a resolution that produced a plan is a planned run; a request the resolver could not satisfy is a rejected run.
- [x] 2.2 Decide where the rule lives: in the shared Core, read by both resolver lanes.
- [x] 2.3 Decide the options and reject the ones that keep a completed action on the record, copy the rule into two lanes or put it in the wrong layer.
- [x] 2.4 Decide the untouched surfaces: the resolvers, the catalogs, the install path, the qualify lanes, the release and deploy lanes, the registry schema, the portal.

## 3. Implement

- [x] 3.1 `src/core/resolve.rs`: the one shared rule, `ResolveRun`, with the journal state, the label and the decision.
- [x] 3.2 `src/core/mod.rs`: the Core publishes the rule.
- [x] 3.3 `src/cli/component.rs`: the component resolver lane reads the rule; the detail names the run.
- [x] 3.4 `src/cli/ui_pattern.rs`: the UI pattern resolver lane reads the same rule; the detail names the run.
- [x] 3.5 `tests/resolve_plan_verdict_truth_contract.rs`: the journal state and the named run for a planned resolution and for a rejected one, in both lanes.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A component resolution with a step journals `planned` and names the run.
- [x] 4.4 A UI pattern resolution with a step journals `planned` and names the run.
- [x] 4.5 A resolution that satisfied nothing journals `rejected` in both lanes.
- [x] 4.6 The resolvers, the catalogs, the install path, the qualify lanes, the release and deploy lanes and the registry schema are unchanged.
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
- [x] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capabilities' purposes.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 7. Evidence

All commands were run from the repository root with
`source /tmp/forge-env.sh` (so `CARGO_TARGET_DIR=/home/paul/code/forge/target`).

| Check | Command | Observed |
|---|---|---|
| Build | `cargo build --workspace` | `Finished \`dev\` profile`, exit 0 |
| Core unit tests | `cargo test -p forge --lib core::` | 18 passed; 0 failed (3 of them the new rule's cases) |
| New contract test | `cargo test --test resolve_plan_verdict_truth_contract` | 4 passed; 0 failed |
| Existing contract tests | `cargo test --test component_contract --test ui_pattern_contract` | 15 passed and 33 passed; 0 failed |
| Full suite | `bash scripts/check-tests.sh` | exit 0; 240 `test result:` lines; 4267 passed; 0 failed; 1 ignored |
| Repository Gate | `forge gate --timeout-secs 2400` | exit 0; 8/8; `observed_at` 2026-10-25T03:34:06Z; revision 545b5a22a2015f2a5abd70e0d7c71fc73311613c |
| Lint | `cargo clippy --workspace --all-targets --all-features` | exit 0; 39 warning lines (the pre-change baseline); no warning names a touched file |
| Format | `rustfmt --edition 2021 --check <file>` on each touched file | clean |
| File size | `wc -l` on each touched file | `src/core/resolve.rs` 82, `src/core/mod.rs` 698, `src/cli/component.rs` 236, `src/cli/ui_pattern.rs` 218, `tests/resolve_plan_verdict_truth_contract.rs` 282 |
| Whitespace | `git diff --check` | no finding |
| Spec validation | `node .../openspec.js validate --all --strict --no-interactive` | `Totals: 164 passed, 0 failed (164 items)` |
| Change names | `node scripts/check-openspec-change-names.mjs` | `PASS` |

The contract test fails on the tree before the change: with the three
touched source files reverted, all four cases fail — the rows read
`done` and neither the output nor the detail names a run.

The defect, reproduced on the tree before the change:

```
$ forge --registry /tmp/res-probe/registry.db component resolve --profile rust-web --component paginated-query
(1, 'component', '__component__', 'done', ..., "rust-web resolved=1 rejected=0; resolved 1 component(s) for profile 'rust-web'")
$ forge --registry /tmp/uip-probe/registry.db ui-pattern resolve --profile react-web --pattern billing
(1, 'ui_pattern', '__ui_pattern__', 'done', ..., "react-web resolved=1 rejected=0; resolved 1 ui pattern(s) for profile 'react-web'")
```

The same two commands on the tree after the change:

```
('component', '__component__', 'planned', "rust-web resolved=1 rejected=0 verdict=planned; resolved 1 component(s) for profile 'rust-web'")
('ui_pattern', '__ui_pattern__', 'planned', "react-web resolved=1 rejected=0 verdict=planned; resolved 1 ui pattern(s) for profile 'react-web'")
```
