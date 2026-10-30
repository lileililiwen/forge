## 1. BFS — Baseline and impact coverage

- [x] 1.1 Read the last remote CI run and list every failing job with its cause.
- [x] 1.2 Reproduce `cargo fmt --all -- --check` and `cargo clippy -- -D warnings` locally.
- [x] 1.3 Confirm `forge check` resolves registered projects only and that a scratch project registers.
- [x] 1.4 Confirm the Gate plan's per-check budgets against the default 600 s wrapper.
- [x] 1.5 Confirm no product behaviour, schema or evidence shape has to change.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Install the declared OpenSpec validator in the `governance` job at a pinned version.
- [x] 2.2 Exercise the checker against a generated and registered scratch project in `surfaces`.
- [x] 2.3 Bound the `gate` job to the resolved plan and install the `cargo-audit` binary it invokes.
- [x] 2.4 Apply workspace formatting.
- [x] 2.5 Fix every clippy warning; keep only the two documented structural allows.
- [x] 2.6 Normalize the checker parity test's wall-clock field.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 `cargo fmt --all -- --check` exits 0.
- [x] 3.2 `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0.
- [x] 3.3 The three changed CI jobs parse and name the tools they install.
- [x] 3.4 A generated `rust-web` project registers and `forge check` emits an `alerts` document.
- [x] 3.5 No product crate gains a behaviour change; only formatting, lints and the workflow differ.
- [x] 3.6 The repository suite still passes.

## 4. Verification

- [x] 4.1 Run `node scripts/check-openspec-change-names.mjs`.
- [x] 4.2 Run `openspec validate --all --strict --no-interactive`.
- [x] 4.3 Run `git diff --check`.
- [x] 4.4 Run `forge gate --dry-run`, then the full `forge gate` with a bounded timeout.
- [x] 4.5 Record every command's exact result in the evidence table.
- [x] 4.6 Archive without `--skip-specs`, commit the change, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Remote CI baseline | 5 jobs failed on the last pushed run | `gh run view 38742111197`: `governance`, `surfaces`, `readiness`, `gate`, `contract-parity` failed; `fmt`, `clippy`, `test`, `native-scaffold`, `msrv`, `deny`, `artifact` passed. 50 commits have landed since. |
| Formatting | PASS | `cargo fmt --all -- --check` exited 0 after `cargo fmt --all`; 14 files were reformatted, including `crates/cargo-audit-boost/*`, `crates/cargo-sonar/*`, `src/lib.rs` and `src/cli/use_cmd.rs`. |
| Lints | PASS | `cargo clippy --workspace --all-targets --all-features -- -D warnings` exited 0 (previously 101 with 21 warnings). Two structural `too_many_arguments` allows remain, on `draw_ui` and `cmd_build`, each with its reason. |
| Repository test suite | PASS, 4361 tests / 0 failed | `bash scripts/check-tests.sh` exited 0; 251 suites reported `ok`, the Rust total was 4361 passed, and the script's Python import checks and `modules/identity-providers` workspace ran after it. |
| Flaky checker parity test | PASS, 5 consecutive runs | `format_json_does_not_alter_checker_stdout` failed once by comparing two `generated_at` values one second apart. The test now replaces that value with `normalized`; `cargo test --test checker_contract` reported 14 passed on five consecutive runs. |
| Checker surface reproduction | PASS | `forge new /tmp/opencode/forge-surfaces --profile rust-web` → `registered forge-surfaces` → `forge check forge-surfaces --format json` parsed to a document with `alerts` (1 alert, the absent-git warning). |
| CI workflow parse | PASS | `python3 -c "yaml.safe_load(open('.github/workflows/ci.yml'))"` parsed; the diff shows the pinned OpenSpec install, the scratch checker path, `cargo-audit@0.22.2` and `--timeout-secs 2400`. |
| Repository Gate | FAIL, 7/8 — pre-existing advisory only | `forge gate . --timeout-secs 1800` exited 1 with aggregate `blocked` at `2026-10-30T12:57:12Z` on revision `2a07a56e9f8a`. `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `source-file-size` (465/465) and `tests` passed; only `security` failed on RUSTSEC-2026-0387 (`crypto-bigint` 0.5.5). This change touches neither `Cargo.toml` nor `Cargo.lock`; remediation is the next change (`dependency-advisory-remediation`). |
| OpenSpec and whitespace | PASS | `node scripts/check-openspec-change-names.mjs` reported `PASS`; `openspec validate --all --strict --no-interactive` reported `Totals: 173 passed, 0 failed (173 items)`; `git diff --check` reported no findings. |

