# automated-size-split-batch Specification

## Purpose

Bring every remaining oversized `src/` file under the 1,000-line
file-size cap by splitting each into a same-location module tree
with splitrs (bodies verbatim, public paths identical via
re-exports, generic bucket names renamed to domain-meaningful
module names), so the Gate `source-file-size` check goes green
with zero behavior change.

## ADDED Requirements

### Requirement: Every oversized src file moves under the cap with paths stable

Forge SHALL split each of the 45 `src/` files over 1,000 lines
(`src/portfolio/interest/activation.rs`,
`src/publish/inventory.rs`, `src/github/cli.rs`,
`src/release/mod.rs`, `src/deploy/mod.rs`, `src/kit/assets.rs`,
`src/semantic/review.rs`, `src/api/fleet.rs`,
`src/doctor/gaps.rs`, `src/publish/mod.rs`,
`src/standard/mod.rs`, `src/github/adapter.rs`,
`src/upgrade/mod.rs`, `src/fleet/mod.rs`, `src/spec/mod.rs`,
`src/profile/mod.rs`, `src/feature/mod.rs`,
`src/deploy/engine.rs`, `src/governance.rs`,
`src/registry/mod.rs`, `src/import/mod.rs`,
`src/publish/providers.rs`, `src/distribution/mod.rs`,
`src/planner/mod.rs`, `src/gate/mod.rs`,
`src/registry/portfolio.rs`, `src/component/mod.rs`,
`src/procedure/mod.rs`, `src/analytics/mod.rs`,
`src/docs/mod.rs`, `src/publish/remote_compose.rs`,
`src/policy/mod.rs`, `src/generate/mod.rs`,
`src/ui_pattern/mod.rs`, `src/mcp/mod.rs`, `src/portal/mod.rs`,
`src/agent/mod.rs`, `src/identity/mod.rs`,
`src/release/engine.rs`, `src/provider/mod.rs`,
`src/api/admin.rs`, `src/api/command_catalog.rs`,
`src/doctor/mod.rs`, `src/api/mod.rs`, `src/main.rs`)
into a same-location module tree (`mod.rs` inputs split inside
their own module dir; leaf files become same-stem sibling dirs
with the original removed per splitrs convention), with each
generated `mod.rs` declaring its submodules and re-exporting
every moved name at its exact prior visibility, so that each
`crate::<path>::<Item>` path resolves exactly as before. Every
moved body SHALL be copied verbatim with no cleanup, no item
renames, and no caller edits. Every `src/**/*.rs` file SHALL be
under 1,000 lines after the batch (including the absorbed
`src/registry/interest/` tree).

#### Scenario: Each split tree holds one item family per domain-named file

- **WHEN** an operator inspects any split module tree
- **THEN** no file is named `functions.rs`, `types.rs`,
  `constants.rs`, `helpers.rs`, `utils.rs`, or `chunk_N.rs`
  (each is renamed to a domain-meaningful snake_case name
  derived from its contents), no two modules in one dir share
  a name, and each file holds one coherent item family with
  verbatim bodies

#### Scenario: Public API, contracts, CLI, and JSON stay byte-identical

- **WHEN** this change is applied on top of the prior `main`
- **THEN** every `pub`/`pub(crate)` path, re-export, contract
  string, validation message, CLI output, JSON shape, and
  behavior SHALL be byte-identical to before the change, and
  the constrained `cargo build` reports 0 errors

### Requirement: Test counts and outcomes are unchanged

Forge SHALL keep every test target green with identical
passed / failed / ignored counts before and after the batch,
with no test added, deleted, renamed, or re-logicked; any
count delta SHALL be proven pre-existing via a stash
baseline.

#### Scenario: Full workspace suite matches the pre-split baseline

- **WHEN** the pre-split and post-split `cargo test
  --workspace --all-targets` summaries are compared
- **THEN** the passed / failed / ignored counts per target
  are identical, or each delta is demonstrated identical on
  the pristine stash baseline
