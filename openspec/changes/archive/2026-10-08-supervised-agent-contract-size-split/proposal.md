# Proposal: Supervised agent contract size split

## Why

`tests/supervised_agent_contract.rs` is 1,201 lines, over the
1,000-physical-line cap enforced by `forge gate`
(`source-file-size`). This is file 2 of the source-file-size grind
per
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9, which pins `tests/supervised_agent_contract.rs` to the
per-provider scenario-group axis (legacy session, ariadex,
sisyphusfy, native toolchain). The working precedents are the
archived slices (`tests/kit_contract/` as `main.rs` + focused
submodules, and `2026-10-08-gate-contract-size-split` with the same
shape: `pub(crate)` visibility, since the integration crate is its
own root).

## What Changes

- `tests/supervised_agent_contract.rs` (1,201 lines) →
  `tests/supervised_agent_contract/` (directory with `main.rs` +
  four focused submodules: `legacy_session`, `ariadex`,
  `sisyphusfy`, `native_toolchain`).
- `main.rs` keeps the file-level doc comment, every shared helper
  (`forge_bin`, `clean_cmd`, `run_env`, `run_json`, `lossy`,
  `evidence_of`, `write_rust_project`, `write_legacy_session`,
  `write_script`, `write_ariadex_stub`, `write_sisyphusfy_stub`,
  the `Fixture` struct and its impl, `start_args`,
  `write_bound_spec`, `outcome_doc`, `code_of`) and the four
  `mod <area>;` declarations. Each submodule owns the related
  `#[test]` functions with bodies copied verbatim.
- No `src/` change, no `mod` declaration change in `src/lib.rs` or
  `src/main.rs`, no public symbol, route, CLI argument, catalog row,
  or env var change. All 29 `#[test]` functions keep their names
  and run once each in the same `supervised_agent_contract`
  target.

## Package Boundary and Split Assessment

One independently verifiable outcome: the
`supervised_agent_contract` test target moves under the
1,000-line cap per file with zero behavior change.
`cargo test --test supervised_agent_contract` reports identical
passed/ignored counts before and after
(29 passed / 0 failed / 0 ignored).

| Package | Single outcome | Boundary / contract | Independent oracle | Owned by this change |
|---|---|---|---|---|
| `supervised-agent-contract-size-split` (**this**) | `tests/supervised_agent_contract.rs` moves under the cap as `main.rs` + 4 submodules, 29 tests verbatim | `tests/supervised_agent_contract/` | `cargo test --test supervised_agent_contract` (29 passed / 0 failed / 0 ignored, same before/after) | yes |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Reusable code / contract | Compatibility gap | Owner | Decision |
|---|---|---|---|---|
| `tests/supervised_agent_contract.rs` (1201) | Single integration test binary; converts to a directory via `main.rs` (Rust integration tests accept a directory) | None — only `fn` → `pub(crate) fn` visibility (integration crate is its own root) and `mod` declarations | `tests/` owns the target | **convert in this change** |
| `tests/kit_contract/` | Working precedent (`main.rs` + submodules, `pub(crate)`, `use super::*;`) | None | `tests/` | **follow, do not touch** |
| `tests/gate_contract/` | Same-shape precedent (file 1 of this grind, 28 tests verbatim) | None | `tests/` | **follow, do not touch** |
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
  `cargo test --test supervised_agent_contract` discovers the target
  the same way.
- **Failure / boundary behavior:** unchanged.
- **Tests:** the 29 existing tests re-run under the same names with
  the same counts; no test added, deleted, renamed, or merged.
- **Privacy / security:** unchanged — moved code is byte-identical.

## Capabilities

- `supervised-agent-contract-size-split`: the
  `supervised_agent_contract` test target moves under the
  1,000-line cap with all test bodies verbatim.

## Non-goals

- No `src/` change of any kind.
- No test-body cleanup, rename, deletion, or merge.
- No public API, route, CLI, catalog, journal, or env change.
- No other file split in this change.
