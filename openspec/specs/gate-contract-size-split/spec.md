# gate-contract-size-split Specification

## Purpose
Keep the gate contract suite under the file-size cap by splitting it into scenario submodules with every test body verbatim and identical counts.
## Requirements
### Requirement: Gate contract target moves under the line cap with bodies verbatim

Forge SHALL convert `tests/gate_contract.rs` (1,157 lines) to
`tests/gate_contract/` (directory with `main.rs` plus five focused
submodules `documented_help` / `passing` / `blocked` /
`review_required` / `unknown_runtime`). `main.rs` SHALL hold the
file doc, every shared helper, the `GateRun` struct, and the five
`mod <area>;` declarations. The crate's `mod ...;` declarations in
`src/lib.rs` and `src/main.rs` SHALL NOT change; the only
filesystem move SHALL be the file-to-directory shape of this one
test target. Every `#[test]` function body SHALL be copied
verbatim with no cleanup and no renames.

#### Scenario: Each submodule owns one scenario family

- **WHEN** an operator inspects `tests/gate_contract/`
- **THEN** `main.rs` holds the file doc, shared helpers, and mod
  declarations, and the five submodules contain the related
  `#[test]` functions: documented-help (5), passing (7), blocked
  (7), review-required (1), unknown-runtime (8)

#### Scenario: Public API, routes, catalog, journal, CLI, and env stay byte-identical

- **WHEN** this change is applied on top of the prior `main`
- **THEN** no `src/` file changes, and the `forge --version`
  output, every `forge <verb> --help` output, every
  `GET /v1/admin/...` envelope, every catalog row, every
  registry/journal row, every CLI exit code, and every
  environment variable name SHALL be byte-identical to before
  the change

#### Scenario: Test count per target is unchanged

- **WHEN** `cargo test --test gate_contract` is run
- **THEN** the target reports 28 passed / 0 failed / 0 ignored,
  the same 28 `#[test]` functions with the same names as before
  the move, each running once

