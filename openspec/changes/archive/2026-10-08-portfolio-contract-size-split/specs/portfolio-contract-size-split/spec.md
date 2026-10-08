# portfolio-contract-size-split (delta)

## Purpose

Keep the portfolio contract suite under the file-size cap by
splitting it into CLI and HTTP submodules with every test body
verbatim and identical counts.

## ADDED Requirements

### Requirement: Portfolio contract target moves under the line cap with bodies verbatim

Forge SHALL convert `tests/portfolio_contract.rs`
(1,261 lines) to `tests/portfolio_contract/` (directory with
`main.rs` plus two focused submodules `cli` / `http`). `main.rs`
SHALL hold the file doc, every shared helper and seed fixture,
and the two `mod <area>;` declarations. The crate's `mod ...;`
declarations in `src/lib.rs` and `src/main.rs` SHALL NOT change;
the only filesystem move SHALL be the file-to-directory shape of
this one test target. Every `#[test]` function body SHALL be
copied verbatim with no cleanup and no renames.

#### Scenario: Each submodule owns one CLI-vs-HTTP group

- **WHEN** an operator inspects `tests/portfolio_contract/`
- **THEN** `main.rs` holds the file doc, shared helpers, seed
  fixtures, and mod declarations, and the two submodules contain
  the related `#[test]` functions: CLI surface (17),
  HTTP authorization-boundary contract plus registry-level
  portal projection (6)

#### Scenario: Public API, routes, catalog, journal, CLI, and env stay byte-identical

- **WHEN** this change is applied on top of the prior `main`
- **THEN** no `src/` file changes, and the `forge --version`
  output, every `forge <verb> --help` output, every
  `GET /v1/admin/...` envelope, every catalog row, every
  registry/journal row, every CLI exit code, and every
  environment variable name SHALL be byte-identical to before
  the change

#### Scenario: Test count per target is unchanged

- **WHEN** `cargo test --test portfolio_contract` is run
- **THEN** the target reports 23 passed / 0 failed / 0 ignored,
  the same 23 `#[test]` functions with the same names as before
  the move, each running once
