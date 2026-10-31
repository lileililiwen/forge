## 1. BFS — Baseline and impact coverage

- [x] 1.1 Reproduce the policy-plane `UNAVAILABLE` on this checkout.
- [x] 1.2 Confirm the in-tree binary answers the adapter command.
- [x] 1.3 Read the resolution order and every caller of `resolve_binary`.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add the source-checkout fallback for `target/{debug,release}/driftwatchdog`.
- [x] 2.2 Extend the refusal to name the checkout paths.
- [x] 2.3 Add the checkout fallback test.
- [x] 2.4 Select the checker surface for every project and count configured checkers.
- [x] 2.5 Treat an empty configuration as a warning, not unavailable.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 `PATH` preference and override behaviour are unchanged.
- [x] 3.2 A checkout without a built runtime keeps the unavailable outcome.
- [x] 3.3 `forge doctor .` no longer reports the policy plane unavailable here.

## 4. Verification

- [x] 4.1 Run the repository test suite.
- [x] 4.2 Run formatting, clippy, OpenSpec validation and whitespace checks.
- [x] 4.3 Run `forge gate --dry-run`, then the full Gate.
- [x] 4.4 Record every command's exact result in the evidence table.
- [x] 4.5 Archive without `--skip-specs`, commit, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| In-tree runtime resolution | PASS | With no `PATH` edit, `forge doctor .` selected `<checkout>/target/debug/driftwatchdog`, ran the checker composition in ~0.07 s, and reported `[WARN] driftwatch-policy ... ran with no configured checkers`. |
| Doctor verdict | PASS | `forge doctor .` now reports `verdict: healthy`; the profile findings remain `PASS`, and the policy plane is a warning rather than `UNAVAILABLE`. |
| Checker surface | PASS | The policy adapter sends `check --dry-run --format json` for both a plain and a gate-managed project; `driftwatch_cli_contract` 8 passed and `driftwatch_cli_alignment_contract` 10 passed. |
| Empty configuration | PASS | `cargo test -p forge --lib no_configured_checkers` passed (2 tests); a report with `checkers = 0` is `NotConfigured` / `WARN`; a configured rule that does not apply remains `Unjudged` / `UNAVAILABLE`. |
| Resolution order | PASS | `cargo test -p forge --lib policy::` passed 57 tests, including the checkout fallback, `PATH` preference and override behaviour. |
| Release mapping | PASS | `release_driftwatch_check_truth_contract` passed; `NotConfigured` maps to a disabled release check, not a block. |
| Repository test suite | PASS, 4380 tests / 0 failed | `bash scripts/check-tests.sh` exited 0 across 253 suites. |
| Formatting and lints | PASS | `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` exited 0. |
| Repository Gate | PASS, 8/8 | `forge gate . --timeout-secs 1800` exited 0 with aggregate `passed` at `2026-10-31T08:41:22Z`. |
| OpenSpec and whitespace | PASS | Name preflight `PASS`; strict validation `173 passed, 0 failed`; `check-spec-governance` `PASS`; `git diff --check` clean. |

