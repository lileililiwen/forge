## 1. BFS — Baseline and impact coverage

- [x] 1.1 Enumerate every advisory and its fixed release.
- [x] 1.2 Confirm the `crypto-bigint` path and that Forge performs verification only.
- [x] 1.3 Confirm `ratatui 0.30.2` removes `lru 0.12` and `paste`.
- [x] 1.4 Confirm both policy files and every job that reads them.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Update `tokio` to `1.54.1` with a single-crate update.
- [x] 2.2 Move both TUI crates to `ratatui 0.30` and adapt the code.
- [x] 2.3 Record the `crypto-bigint` exception in `deny.toml` with its reason.
- [x] 2.4 Record the same exception in `.cargo/audit.toml`.
- [x] 2.5 Add the policy parity contract test.
- [x] 2.6 Version the `driftwatchdog` path dependency so the bans check passes.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 `cargo tree -i lru` and `cargo tree -i paste` report no package.
- [x] 3.2 `cargo audit` exits 0.
- [x] 3.3 The two TUI crates compile and their tests pass.
- [x] 3.4 The policy parity test rejects a one-file exception.
- [x] 3.5 The identity surface is unchanged and its tests pass.

## 4. Verification

- [x] 4.1 Run the repository test suite.
- [x] 4.2 Run formatting, clippy, OpenSpec validation and whitespace checks.
- [x] 4.3 Run `forge gate --dry-run`, then the full Gate.
- [x] 4.4 Record every command's exact result in the evidence table.
- [x] 4.5 Archive without `--skip-specs`, commit, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Advisory inventory | 1 vulnerability + 5 warnings before; 0 vulnerabilities + 1 allowed warning after | Before: `RUSTSEC-2026-0387` (`crypto-bigint`), `RUSTSEC-2026-0386` (`tokio`), `RUSTSEC-2026-0253` and `RUSTSEC-2026-0002` (`lru`), `RUSTSEC-2024-0436` (`paste`), `RUSTSEC-2024-0449` (`unsafe-libyaml`). After: `cargo audit` reports only the allowed `unsafe-libyaml` warning and exits 0. |
| `tokio` upgrade | PASS | `cargo update -p tokio --precise 1.54.1` moved `tokio 1.53.2 -> 1.54.1`; `cargo tree -i tokio` names `1.54.1`. |
| `ratatui` upgrade | PASS | Both TUI crates declare `ratatui = "0.30"`; `cargo tree -i lru` and `cargo tree -i paste` report `did not match any packages`; the only source changes were `Frame::size()` → `Frame::area()` and `Table::highlight_style` → `row_highlight_style`. |
| TUI crate tests | PASS, 6 tests / 0 failed | `cargo test -p cargo-audit-boost -p cargo-matrix-boost` reported 6 passed across 2 suites. |
| `cargo audit` | PASS | Exited 0, scanning 469 dependencies; the only remaining entry is the already-excepted `unsafe-libyaml` warning. |
| `cargo deny check` | PASS | `cargo-deny 0.20.2` reported `advisories ok, bans ok, licenses ok, sources ok`. The `bans` failure was a wildcard: `driftwatchdog = { path = "crates/driftwatchdog" }` had no version; it now carries `version = "0.1.11"`. |
| Policy parity test | PASS, 5 tests / 0 failed | `cargo test --test dependency_policy_contract` verifies both files name `RUSTSEC-2023-0071` and `RUSTSEC-2026-0387`, every `deny.toml` ignore has a reason, the `crypto-bigint` reason names `openidconnect`, `public-key`, `verification`, `no private-key` and the upstream trigger, and the lockfile has not reinstated `paste 1.0.15`, `lru 0.12.5` or `tokio 1.53.2`. |
| MSRV floor | PASS | `cargo +1.88.0 check --workspace --all-targets` exited 0 in 56.22s; `ratatui 0.30.2` declares `rust-version = 1.88.0`, the repository floor. |
| Repository test suite | PASS, 4366 tests / 0 failed | `bash scripts/check-tests.sh` exited 0; 252 suites reported `ok`, the Rust total was 4366 passed, and the script's Python checks and `modules/identity-providers` workspace ran after it. |
| Formatting and lints | PASS | `cargo fmt --all -- --check` exited 0; `cargo clippy --workspace --all-targets --all-features -- -D warnings` exited 0. |
| Repository Gate | PASS, 8/8 | `forge gate . --timeout-secs 1800` exited 0 with aggregate `passed` at `2026-10-30T13:35:45Z` on revision `c0162a19`. `security` passed with the `unsafe-libyaml` warning only; every other check passed. This is the first passing repository Gate recorded in the recent cycles. |
| OpenSpec and whitespace | PASS | `node scripts/check-openspec-change-names.mjs` reported `PASS`; `openspec validate --all --strict --no-interactive` reported `Totals: 173 passed, 0 failed (173 items)`; `check-spec-governance` reported `PASS`; `git diff --check` reported no findings. |

