# supervised-agent-contract-size-split Specification

## Purpose
Keep the supervised-agent contract suite under the file-size cap by splitting it into per-provider scenario submodules with every test body verbatim and identical counts.
## Requirements
### Requirement: Supervised agent contract target moves under the line cap with bodies verbatim

Forge SHALL convert `tests/supervised_agent_contract.rs`
(1,201 lines) to `tests/supervised_agent_contract/` (directory
with `main.rs` plus four focused submodules `legacy_session` /
`ariadex` / `sisyphusfy` / `native_toolchain`). `main.rs` SHALL
hold the file doc, every shared helper, the `Fixture` struct and
its impl, and the four `mod <area>;` declarations. The crate's
`mod ...;` declarations in `src/lib.rs` and `src/main.rs` SHALL NOT
change; the only filesystem move SHALL be the file-to-directory
shape of this one test target. Every `#[test]` function body SHALL
be copied verbatim with no cleanup and no renames.

#### Scenario: Each submodule owns one provider scenario group

- **WHEN** an operator inspects `tests/supervised_agent_contract/`
- **THEN** `main.rs` holds the file doc, shared helpers, and mod
  declarations, and the four submodules contain the related
  `#[test]` functions: legacy-session (3), ariadex (12),
  sisyphusfy (9), native-toolchain (5)

#### Scenario: Public API, routes, catalog, journal, CLI, and env stay byte-identical

- **WHEN** this change is applied on top of the prior `main`
- **THEN** no `src/` file changes, and the `forge --version`
  output, every `forge <verb> --help` output, every
  `GET /v1/admin/...` envelope, every catalog row, every
  registry/journal row, every CLI exit code, and every
  environment variable name SHALL be byte-identical to before
  the change

#### Scenario: Test count per target is unchanged

- **WHEN** `cargo test --test supervised_agent_contract` is run
- **THEN** the target reports 29 passed / 0 failed / 0 ignored,
  the same 29 `#[test]` functions with the same names as before
  the move, each running once

