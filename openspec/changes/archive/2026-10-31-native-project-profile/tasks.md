## 1. BFS — Baseline and impact coverage

- [x] 1.1 Reproduce `forge doctor .` and `forge register .` on this repository.
- [x] 1.2 Map every `ProfileSupportStatus` branch and every doctor profile lookup.
- [x] 1.3 Confirm generation refuses planned profiles before staging.
- [x] 1.4 Confirm registration goes through `resolve_profile`.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add `Native` and the `rust-product` descriptor; include it in inspect.
- [x] 2.2 Allow registration resolution and refuse capability requests.
- [x] 2.3 Refuse generation for every non-supported status with a specific native message.
- [x] 2.4 Judge the native build definition and feature compatibility in doctor.
- [x] 2.5 Stop reporting a native profile as a planned readiness gap.
- [x] 2.6 Register and check this repository in the CI surfaces job.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 `list_profiles` still contains only supported profiles.
- [x] 3.2 `forge new --profile rust-product` is refused and writes nothing.
- [x] 3.3 `forge doctor .` reports `profile-known PASS` and `features-compatible PASS`.
- [x] 3.4 `forge register .` succeeds and `forge check forge` emits `alerts`.
- [x] 3.5 Unknown-profile behaviour is unchanged.

## 4. Verification

- [x] 4.1 Run the repository test suite.
- [x] 4.2 Run formatting, clippy, OpenSpec validation and whitespace checks.
- [x] 4.3 Run `forge gate --dry-run`, then the full Gate.
- [x] 4.4 Record every command's exact result in the evidence table.
- [x] 4.5 Archive without `--skip-specs`, commit, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Native descriptor | PASS | `forge profile inspect rust-product` reports `support_status: native`; `list_profiles` still contains only the six supported profiles. |
| Registration | PASS | `forge register .` registered `forge (/home/paul/code/forge)`; `forge check forge --format json` emitted a `forge-checker/0.1.0` document with alerts. |
| Generation refusal | PASS | `forge new /tmp/opencode/native-probe --profile rust-product` exited 1 with `unsupported-profile` naming the native stack; the destination was not created. |
| Doctor assessment | PASS | `cargo test -p forge --lib native_profile_is_known_and_assessed` passed: `profile-known`, `features-compatible` and `build-config` are all `PASS` for a native project with `Cargo.toml`. |
| Real doctor | PASS for profile findings | `forge doctor .` reports `[PASS] profile-known` and `[PASS] features-compatible`; the remaining findings are a declaration-vocabulary warning, a gate-evidence warning, a release-evidence warning and the optional policy plane `UNAVAILABLE` (no driftwatch binary). |
| Evidence claim | Corrected | `.project.json` now records `evidence_status: implemented`; doctor's `release-evidence` finding is a warning rather than the assertion-without-evidence FAIL it reported for `verified`. |
| CI dogfooding | Updated | The `surfaces` job runs `forge doctor .`, `forge register .` and `forge check forge` on this repository instead of a scratch project. |
| Source-size baseline | PASS | `src/doctor/doctor_tests.rs` was over 1,000 lines after the new test; the test moved to `src/doctor/doctor_native_tests.rs` under `#[cfg(test)]`, and `product_code_quality.py` passes. |
| Repository test suite | PASS, 4377 tests / 0 failed | `bash scripts/check-tests.sh` exited 0 across 253 suites. |
| Formatting and lints | PASS | `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` exited 0. |
| Repository Gate | PASS, 8/8 | `forge gate . --timeout-secs 1800` exited 0 with aggregate `passed` at `2026-10-31T06:38:18Z`. |
| OpenSpec and whitespace | PASS | Name preflight `PASS`; strict validation `173 passed, 0 failed`; `check-spec-governance` `PASS`; `git diff --check` clean. |

