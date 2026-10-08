# Proposal: Portfolio contract size split

## Why

`tests/portfolio_contract.rs` is 1,261 lines, over the
1,000-physical-line cap enforced by `forge gate`
(`source-file-size`). This is file 3 of the source-file-size grind
per
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9, which pins `tests/portfolio_contract.rs` to the CLI-vs-HTTP
axis (helpers + seed fixtures into `main.rs`). The working
precedents are the archived slices (`tests/kit_contract/` as
`main.rs` + focused submodules, `2026-10-08-gate-contract-size-split`
and `2026-10-08-supervised-agent-contract-size-split` with the same
shape: `pub(crate)` visibility, since the integration crate is its
own root).

## What Changes

- `tests/portfolio_contract.rs` (1,261 lines) →
  `tests/portfolio_contract/` (directory with `main.rs` + two
  focused submodules: `cli`, `http`).
- `main.rs` keeps the file-level doc comment, every shared helper
  (`forge_bin`, `lossy`, `clean_cmd`, `run`, `run_json`,
  `write_identity_project`, `registered`, `mint_session_token`,
  `drive`, `api_request`, `api_json`, `seed_two_projects`) and the
  two `mod <area>;` declarations. Each submodule owns the related
  `#[test]` functions with bodies copied verbatim.
- No `src/` change, no `mod` declaration change in `src/lib.rs` or
  `src/main.rs`, no public symbol, route, CLI argument, catalog row,
  or env var change. All 23 `#[test]` functions keep their names
  and run once each in the same `portfolio_contract` target.

## Package Boundary and Split Assessment

One independently verifiable outcome: the `portfolio_contract`
test target moves under the 1,000-line cap per file with zero
behavior change. `cargo test --test portfolio_contract` reports
identical passed/ignored counts before and after
(23 passed / 0 failed / 0 ignored).

| Package | Single outcome | Boundary / contract | Independent oracle | Owned by this change |
|---|---|---|---|---|
| `portfolio-contract-size-split` (**this**) | `tests/portfolio_contract.rs` moves under the cap as `main.rs` + 2 submodules, 23 tests verbatim | `tests/portfolio_contract/` | `cargo test --test portfolio_contract` (23 passed / 0 failed / 0 ignored, same before/after) | yes |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Reusable code / contract | Compatibility gap | Owner | Decision |
|---|---|---|---|---|
| `tests/portfolio_contract.rs` (1261) | Single integration test binary; converts to a directory via `main.rs` (Rust integration tests accept a directory) | None — only `fn` → `pub(crate) fn` visibility (integration crate is its own root) and `mod` declarations | `tests/` owns the target | **convert in this change** |
| `tests/kit_contract/` | Working precedent (`main.rs` + submodules, `pub(crate)`, `use super::*;`) | None | `tests/` | **follow, do not touch** |
| `tests/gate_contract/` | Same-shape precedent (file 1 of this grind, 28 tests verbatim) | None | `tests/` | **follow, do not touch** |
| `tests/supervised_agent_contract/` | Same-shape precedent (file 2 of this grind, 29 tests verbatim) | None | `tests/` | **follow, do not touch** |
| Every `src/` file | Untouched by this change | N/A | `src/` | **defer** (later grind files) |

## User Experience and Interface Impact

`UI/UX: N/A` — pure test-file move. No CLI, API, frontend, journal,
catalog, env, or `--version` change. The operator sees no difference.

## BFS Impact Map

- **Capabilities:** none new; one `tests/` file moves under the cap.
- **Users / flows:** unchanged.
- **Contracts / data / persistence:** no schema, route, command,
  journal, or env change.
- **Integrations / configuration:** none.
- **Callers:** no caller-side change;
  `cargo test --test portfolio_contract` discovers the target the
  same way.
- **Failure / boundary behavior:** unchanged.
- **Tests:** the 23 existing tests re-run under the same names with
  the same counts; no test added, deleted, renamed, or merged.
- **Privacy / security:** unchanged — moved code is byte-identical.

## Capabilities

- `portfolio-contract-size-split`: the `portfolio_contract` test
  target moves under the 1,000-line cap with all test bodies
  verbatim.

## Non-goals

- No `src/` change of any kind.
- No test-body cleanup, rename, deletion, or merge.
- No public API, route, CLI, catalog, journal, or env change.
- No other file split in this change.
