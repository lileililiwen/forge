# Tasks: component-qualify-evidence-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the qualify path: `src/cli/component.rs`, `src/cli/commands_ops.rs` (the `Qualify` flags).
- [x] 1.3 Read the core: `src/component/model.rs`, `src/component/engine.rs`, `src/component/contract.rs`, `src/component/catalog.rs`.
- [x] 1.4 Read the canonical capability: `openspec/specs/semantic-component-registry/spec.md`.
- [x] 1.5 Read the existing tests: `tests/component_contract.rs`, `src/component/engine_tests.rs`.
- [x] 1.6 Reproduce the defect on the current tree and record the receipt it wrote.

## 2. DFS requirement implementation

- [x] 2.1 Decide the rule: a promotion records only the evidence the operator supplied; an absent field is absent.
- [x] 2.2 Decide where the rule lives: in the core certified evidence gate, read by every promotion writer.
- [x] 2.3 Decide the options for a certified promotion without complete evidence and reject the ones that keep a false claim, split one refusal class in two or invent a measurement Forge cannot make.
- [x] 2.4 Decide the untouched surfaces: the catalog, the resolver, the deprecated path, the receipt layout, the journal.

## 3. Implement

- [x] 3.1 `src/component/model.rs`: `ComponentQualifyEvidence` holds optional evidence; `ComponentQualifyOutcome` names the missing evidence.
- [x] 3.2 `src/component/engine.rs`: one certified evidence gate that refuses an absent field; the receipt carries only the supplied evidence.
- [x] 3.3 `src/cli/component.rs`: the flags are passed through with no default coverage, timestamp or security review; the output publishes the missing evidence.
- [x] 3.4 `src/cli/commands_ops.rs`: the help no longer advertises defaults that do not exist.
- [x] 3.5 `src/component/engine_tests.rs` and `tests/component_contract.rs`: the evidence constructions follow the optional fields; the complete-evidence case supplies its evidence.
- [x] 3.6 `tests/component_qualify_evidence_truth_contract.rs`: the CLI contract for a promotion without evidence, with complete evidence and for a receipt that carries nothing invented.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 A certified promotion without evidence is refused, names the missing evidence, keeps the prior level and writes no receipt.
- [x] 4.4 A certified promotion with complete evidence is accepted and the receipt records exactly those values.
- [x] 4.5 A receipt carries no field the operator did not supply.
- [x] 4.6 The deprecated path, the no-op path, the invalid-value path and the journal are unchanged.
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
| Component unit tests | `cargo test -p forge --lib component::` | 27 passed; 0 failed |
| New contract test | `cargo test --test component_qualify_evidence_truth_contract` | 3 passed; 0 failed |
| Component contract test | `cargo test --test component_contract` | 15 passed; 0 failed |
| Full suite | `bash scripts/check-tests.sh` | exit 0; 239 `test result:` lines; 4260 passed; 0 failed; 1 ignored |
| Repository Gate | `forge gate --timeout-secs 2400` | exit 0; 8/8; `observed_at` 2026-10-24T16:08:33Z; revision 36c66a43c56b9f6ca670ea3e8b145c5042a7a333 |
| Lint | `cargo clippy --workspace --all-targets --all-features` | exit 0; 39 warning lines (the pre-change baseline); no warning names a touched file |
| Format | `rustfmt --edition 2021 --check <file>` on each touched file | clean |
| File size | `wc -l` on each touched file | `src/cli/commands_ops.rs` 672, `src/cli/component.rs` 233, `src/component/engine.rs` 536, `src/component/engine_tests.rs` 557, `src/component/model.rs` 218, `tests/component_contract.rs` 492, `tests/component_qualify_evidence_truth_contract.rs` 185 |
| Whitespace | `git diff --check` | no finding |
| Spec validation | `node .../openspec.js validate --all --strict --no-interactive` | `Totals: 163 passed, 0 failed (163 items)` |
| Change names | `node scripts/check-openspec-change-names.mjs` | `PASS` |

The contract test fails on the tree before the change: with the six
touched source and test files reverted, all three cases fail — the
promotion is accepted with a receipt that carries a fabricated
`last_verified` of `now`.

The defect, reproduced on the tree before the change:

```
$ forge component qualify toast --to certified --reason "looks fine" --path /tmp/comp-probe
$ cat /tmp/comp-probe/.forge/components/toast/qualify.json
{
  "contract": "0.1.0",
  "component_id": "toast",
  "target_quality": "certified",
  "reason": "looks fine",
  "evidence": {
    "test_coverage": 0.949999988079071,
    "known_issues": [],
    "last_verified": "2026-10-24T15:00:16.482913Z",
    "security_review": true
  }
}
```

The same command on the tree after the change:

```
$ forge component qualify toast --to certified --reason "looks fine" --path /tmp/comp-probe2
promotion refused: missing required evidence: test_coverage, last_verified, security_review; a certified promotion requires the operator to supply it; component 'toast' remains at quality 'verified'
missing evidence: test_coverage, last_verified, security_review
files: (none)
prior: verified target: certified promoted: false
$ find /tmp/comp-probe2 -type f
/tmp/comp-probe2/registry.db
```
