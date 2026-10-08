# Proposal: Source file size remediation

## Why

`forge gate` enforces a 1000-physical-line cap on every Rust source file
under `src/` and `tests/`. Today **50 of 119** evaluated files exceed it
(2,589 → 14,619 lines in the worst cases), so the gate reports
`source-file-size: fail` and blocks every archive even when the change
itself is clean. The cap exists for the same reason every other module
size cap exists in a Rust crate: large single files slow review, make
ownership fuzzy, and hide mechanical change behind the noise.

The remediation needs an explicit package because it is non-trivial,
spans every module family the codebase owns, and must keep the public
API, the in-process semantics, the journal schema, the route table and
the catalog contract all byte-identical. It is also a multi-session
program: every oversized file gets its own decision (where to split,
which helpers to extract, which tests stay where), so this change
authors a **bounded first slice** and leaves the rest of the
remediation on a documented follow-on roadmap.

## What Changes

- A **bounded first slice** of `tests/kit_contract.rs` moves from
  one oversized file to a directory of focused submodules, with
  `main.rs` (or `mod`-style re-export) staying the only entry
  the rest of the crate sees. No public symbol, no journal
  column, no CLI argument, no API route, no catalog row, no
  `forge --version` output, and no environment variable changes.
- The split is proven by `cargo test --test kit_contract`:
  every one of the 60 `#[test]` functions (one of which is
  `#[ignore]`) runs identically to before, and the test count
  is unchanged.
- The follow-on roadmap (the remaining 55 oversized files) is
  pinned in the change's `design.md` so future OpenSpec
  packages can pick up one file at a time without re-deciding
  the strategy.

## Package Boundary and Split Assessment

One independently verifiable outcome: the gate's
`source-file-size` finding's attributable count drops by the number
of files this change brings under the 1000-line cap, and every
existing test target still passes. Splitting one file is one
verifiable move; the gate verdict is the package-level oracle. A
single package that tries to do all 50 in one shot is forbidden by
the workflow (one change → local verify → strict validate → archive),
and would still leave split decisions to do mechanically — the work
belongs in focused, single-file slices that each drop the failing
count and own their own diff.

| Package | Single outcome | Boundary / contract | Independent oracle | Owned by this change |
|---|---|---|---|---|
| `source-file-size-remediation` (**this**) | `tests/kit_contract.rs` (2,376 lines) moves under the 1000-line cap as a directory of 9 submodules, with the 60 `#[test]` functions preserved verbatim and the 59 active tests passing | `tests/kit_contract/` | `cargo test --test kit_contract` (60 tests, 1 `#[ignore]`, same as before the move) | yes |
| `source-file-size-remediation-*` (follow-on, one per file) | One file under the cap, full test target green | `src/<module>/` or `tests/<target>/` | That target's tests | future changes, pinned in `design.md` §9 |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Reusable code / contract | Compatibility gap | Owner | Decision |
|---|---|---|---|---|
| `tests/kit_contract.rs` (2376) | The single integration test binary; converts to a directory `tests/kit_contract/` with submodules (Rust integration tests accept a directory via `main.rs`) | None — every test stays reachable via the same `mod` tree; only `pub(super)` becomes `pub(crate)` (an integration crate has no parent) | `tests/` owns the target | **convert to directory in this change** |
| `tests/portal_ui_contract.rs` (1431) | Single integration test binary; converts to `tests/portal_ui_contract/` with helper + scenario submodules | None | `tests/` | **defer** (follow-on package, pinned in `design.md`) |
| `tests/portfolio_contract.rs` (1261) | Same pattern | None | `tests/` | **defer** |
| `tests/supervised_agent_contract.rs` (1201) | Same pattern | None | `tests/` | **defer** |
| `tests/gate_contract.rs` (1157) | Same pattern | None | `tests/` | **defer** |
| `src/api/admin.rs` (2589) | Already split from 3,822 → 2,589 by `forge-web-workspace-onboarding`; further split into per-verb submodules | None — the crate's `mod admin;` and the `Route::Admin*` enum stay authoritative | `src/api` | **defer** (follow-on package, pinned in `design.md`) |
| `src/release/engine.rs` (2258) | Pure move of helper functions; the in-process `prepare_release` / `apply_release` entry points stay | None | `src/release` | **defer** |
| `src/doctor/mod.rs` (3030) | The diagnostic plane; submodules by concern (gaps, maturity, output) | None | `src/doctor` | **defer** |
| `src/api/command_catalog.rs` (2852) | Typed row builders + Clap-tree parity; submodules by row family | None | `src/api` | **defer** |
| `src/main.rs` (14619) | The CLI binary; subcommand families move to `src/cli/<family>.rs` | None — Clap subcommand derives are pure moves | `src/main.rs` | **defer** (this would be a multi-session slice on its own; pinned in `design.md`) |
| `src/api/mod.rs` (5410) | The route table; submodules by route family | None | `src/api` | **defer** |

## User Experience and Interface Impact

`UI/UX: N/A` — this is a pure module-internal refactor. The terminal
CLI surface, the JSON envelopes, the routes, the catalog rows, the
test names, the env vars, the `--version` output, the registry
schema, the journal schema, the share allowlist, the share
publication, the deploy/release/publish engines, the doctor
projection, the registry/journal persistence — every observable
boundary is byte-identical. The operator and the human-facing
dashboard see no change.

## BFS Impact Map

- **Capabilities:** none new; the `source-file-size` gate finding
  drops by the count of files this change brings under the cap.
- **Users / flows:** unchanged.
- **Contracts / data / persistence:** no schema, route, command,
  journal, or env change. Every existing `cargo test --test ...`
  target stays green with the same test count.
- **Integrations / configuration:** none.
- **Callers:** no caller-side change. The crate's `mod ...;` lines
  keep pointing at the same module name; only the file-vs-directory
  shape flips.
- **Failure / boundary behavior:** unchanged.
- **Tests:** the existing test targets re-run under the same name
  with the same pass count; no test is deleted, renamed, or merged.
- **Privacy / security:** unchanged — the moved code is identical.

## Capabilities

- `source-file-size-remediation`: a bounded first slice of oversized
  Rust files moves under the 1000-line cap with all public boundaries
  byte-identical.

## Non-goals

- No public API change, no new module path surfaced to other crates
  (everything stays `mod <name>;`).
- No test deletion, renaming, or merging.
- No behavior change in any function.
- No follow-on file splits in this change — the remaining
  ~45 oversized files are pinned in `design.md` as one-file-per-
  package follow-on work, not bolted on here.
