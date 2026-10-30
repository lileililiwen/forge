## 1. BFS — Baseline and impact coverage

- [x] 1.1 Compare `forge.yaml`, `.ai-gate/gate.yaml` and the effective plan.
- [x] 1.2 Confirm which promoted spec governs and why they conflict.
- [x] 1.3 Confirm the declared `governance-quality` path does not resolve.
- [x] 1.4 Confirm the fixture no longer matches the live capture.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Remove the stale floor row and set the rule pack in `forge.yaml`.
- [x] 2.2 Add the live declaration/plan parity contract test.
- [x] 2.3 Regenerate the rehearsal fixture and update its assertions.
- [x] 2.4 Remove GDM-008's nine-check requirement and add the effective-floor one.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 The parity test passes on the reconciled declaration.
- [x] 3.2 The parser test passes on the regenerated capture.
- [x] 3.3 A stale declared row fails the parity test (mutate, observe, restore).
- [x] 3.4 The dry-run still reports eight checks, none unbound.

## 4. Verification

- [x] 4.1 Run the repository test suite.
- [x] 4.2 Run formatting, clippy, OpenSpec validation and whitespace checks.
- [x] 4.3 Run `forge gate --dry-run`, then the full Gate.
- [x] 4.4 Record every command's exact result in the evidence table.
- [x] 4.5 Archive without `--skip-specs`, commit, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Declaration/plan drift | Fixed | Before: `forge.yaml` declared nine checks including `governance-quality` (path `forge/modules/workspace-governance/scripts/workspace_check.py`) and `rule_pack: workspace-governance@1`; the plan ran eight under `forge-baseline/1.0.0` and reported `unbound required checks: none`. After: the stale row is removed, the floor and plan are the same eight ids, and the rule pack is `forge-baseline/1.0.0`. |
| Live parity contract | PASS, 5 tests / 0 failed | `cargo test --test gate_declaration_parity` compares `forge.yaml` with `forge gate --dry-run --format json`: id-set equality, rule-pack equality, declared source values, path resolution and an empty unbound list. |
| Parity test catches the regression | PASS | Adding a temporary `stale-check` row made `the_declared_floor_equals_the_effective_plan` fail naming the extra id; removing it returned all five tests to pass. |
| Rehearsal fixture | PASS, 9 tests / 0 failed | `driftwatchdog gate --dry-run --format json` regenerated `tests/fixtures/gate/gate-rehearsal-plan.json` as an eight-check `forge-baseline/1.0.0` capture; `cargo test -p forge --lib gate::rehearsal` passed all nine tests with the updated assertions (two 600 000 ms test checks, six 60 000 ms checks). |
| Dry run | PASS | `forge gate --dry-run --format json` reported `checks: 8`, `rule: forge-baseline/1.0.0`, `unbound: []`. |
| Repository test suite | PASS, 4371 tests / 0 failed | `bash scripts/check-tests.sh` exited 0; 253 suites reported `ok`, the Rust total was 4371 passed, and the script's Python checks and `modules/identity-providers` workspace ran after it. |
| Formatting and lints | PASS | `cargo fmt --all -- --check` exited 0; `cargo clippy --workspace --all-targets --all-features -- -D warnings` exited 0. |
| Repository Gate | PASS, 8/8 | `forge gate . --timeout-secs 1800` exited 0 with aggregate `passed` at `2026-10-30T14:01:57Z`; every check passed. |
| OpenSpec and whitespace | PASS | `node scripts/check-openspec-change-names.mjs` reported `PASS`; `openspec validate --all --strict --no-interactive` reported `Totals: 173 passed, 0 failed`; `check-spec-governance` reported `PASS`; `git diff --check` reported no findings. |

