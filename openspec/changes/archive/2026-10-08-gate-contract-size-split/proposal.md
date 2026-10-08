# Proposal: Gate contract size split

## Why

`tests/gate_contract.rs` is 1,157 lines, over the 1,000-physical-line
cap enforced by `forge gate` (`source-file-size`). This is file 1 of
the source-file-size grind per
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9, which pins `tests/gate_contract.rs` to the
documented-help / passing / blocked / review-required /
unknown-runtime scenario-family axis. The working precedent is the
archived first slice (`tests/kit_contract/` as `main.rs` + focused
submodules with `pub(crate)` visibility, since the integration crate
is its own root).

## What Changes

- `tests/gate_contract.rs` (1,157 lines) → `tests/gate_contract/`
  (directory with `main.rs` + five focused submodules:
  `documented_help`, `passing`, `blocked`, `review_required`,
  `unknown_runtime`).
- `main.rs` keeps the file-level doc comment, every shared helper
  (`forge_bin`, `fixture`, `lossy`, `write_script`, `rust_manifest`,
  `write_file`, `git`, `project`, `GateRun`, `gate`, `gate_human`,
  `gate_json`, `document_stub`, `passing_stub`, `blocked_stub`,
  `journal_for`, `evidence_file`, `head`, plus the
  evidence-export stub helpers `evidence_fixture`,
  `evidence_export_stub_with_project`, `evidence_export_stub`,
  `evidence_unavailable_stub`, `evidence_malformed_stub`,
  `evidence_unknown_field_stub`,
  `evidence_publication_contradiction_stub`, `gate_evidence_cmd`,
  `evidence_json`, `evidence_status_file`) and the five
  `mod <area>;` declarations. Each submodule owns the related
  `#[test]` functions with bodies copied verbatim.
- No `src/` change, no `mod` declaration change in `src/lib.rs` or
  `src/main.rs`, no public symbol, route, CLI argument, catalog row,
  or env var change. All 28 `#[test]` functions keep their names
  and run once each in the same `gate_contract` target.

## Package Boundary and Split Assessment

One independently verifiable outcome: the `gate_contract` test
target moves under the 1,000-line cap per file with zero behavior
change. `cargo test --test gate_contract` reports identical
passed/ignored counts before and after (28 passed / 0 ignored).

| Package | Single outcome | Boundary / contract | Independent oracle | Owned by this change |
|---|---|---|---|---|
| `gate-contract-size-split` (**this**) | `tests/gate_contract.rs` moves under the cap as `main.rs` + 5 submodules, 28 tests verbatim | `tests/gate_contract/` | `cargo test --test gate_contract` (28 passed / 0 ignored, same before/after) | yes |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Reusable code / contract | Compatibility gap | Owner | Decision |
|---|---|---|---|---|
| `tests/gate_contract.rs` (1157) | Single integration test binary; converts to a directory via `main.rs` (Rust integration tests accept a directory) | None — only `fn` → `pub(crate) fn` visibility (integration crate is its own root) and `mod` declarations | `tests/` owns the target | **convert in this change** |
| `tests/kit_contract/` | Working precedent (`main.rs` + submodules, `pub(crate)`, `use super::*;`) | None | `tests/` | **follow, do not touch** |
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
- **Callers:** no caller-side change; `cargo test --test gate_contract`
  discovers the target the same way.
- **Failure / boundary behavior:** unchanged.
- **Tests:** the 28 existing tests re-run under the same names with
  the same counts; no test added, deleted, renamed, or merged.
- **Privacy / security:** unchanged — moved code is byte-identical.

## Capabilities

- `gate-contract-size-split`: the `gate_contract` test target moves
  under the 1,000-line cap with all test bodies verbatim.

## Non-goals

- No `src/` change of any kind.
- No test-body cleanup, rename, deletion, or merge.
- No public API, route, CLI, catalog, journal, or env change.
- No other file split in this change.
