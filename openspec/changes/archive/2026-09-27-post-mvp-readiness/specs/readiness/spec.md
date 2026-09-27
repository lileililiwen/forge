# readiness Specification

## Purpose

Define what "publicly presentable" means for the already-implemented Forge
v0.1: the README maps the registry/import/list/inspect/new/doctor surfaces,
presents the install path as an explicit plan rather than a delivered feature,
ships a real quickstart capture, states the keep-local governance default and
the sibling boundary honestly, and carries a license matching its declared
expression.

## ADDED Requirements

### Requirement: README maps the v0.1 surfaces

The README SHALL describe `forge.yaml`, the project/profile registries and the
commands `forge import`, `forge list`, `forge inspect`, `forge new` and
`forge doctor` as a task-oriented map, each statement traceable to implemented
behavior.

#### Scenario: Command map is audited

- **WHEN** the README command map is compared with the built binary and source
- **THEN** every documented command states its real behavior and no undocumented or fictional surface is named

#### Scenario: Documented behavior has no implementation

- **WHEN** the README describes a command behavior the binary does not have
- **THEN** the text is corrected before readiness is claimed

### Requirement: Installation is presented through the delivered packaging path

The proposal was authored when no Forge installation packaging was
delivered; the `artifact-and-ci-baseline` change archived first (2026-09-27)
and now owns the delivered path. The README SHALL therefore present the
install path through that delivered, digest-verified packaging
(`scripts/package.sh` → `scripts/checksum.sh --verify` →
`scripts/install.sh` → `scripts/smoke.sh`), SHALL name the owning change,
and SHALL NOT present a plan as a delivered feature or imply publication,
tagging or deployment.

#### Scenario: Reader looks for install

- **WHEN** a reader looks for an install path
- **THEN** the README names the delivered scripts, the digest verification step and the owning change, without claiming publication, tagging, push or deploy

### Requirement: Reproducible quickstart demo and terminal capture

The repository SHALL commit a transcript and terminal capture generated from
real runs of the built binary covering import, list, inspect, new and doctor,
referenced from the README.

#### Scenario: Transcript matches the binary

- **WHEN** the transcript commands are re-run against the current binary
- **THEN** the recorded output and behavior match, and any divergence is re-captured from a real run rather than hand-edited

#### Scenario: Capture leaks host information

- **WHEN** a capture would include an absolute host path or a credential
- **THEN** it is excluded or redacted before it is committed

### Requirement: Repository license matches the declared expression

The repository SHALL ship the license text matching the expression declared in
`Cargo.toml`, written exactly once across the active packaging and readiness
changes.

#### Scenario: License declaration has no file

- **WHEN** the manifest declares MIT and no `LICENSE` file exists
- **THEN** readiness fails, and the file is created once rather than duplicated by two changes

### Requirement: Governance keep-local default and sibling boundary

The README SHALL state that the `local` provider is the default with no
`.forge/providers.yaml`, describe the optional `workspace-governance` preset,
and SHALL NOT imply a parent-directory search, a network requirement or a
sibling dependency for local workflows.

#### Scenario: No provider configuration

- **WHEN** a project has no `.forge/providers.yaml`
- **THEN** the README documents that the local provider validates the canonical `forge.yaml` without a sibling, network or external binary

#### Scenario: Sibling prerequisite is unmet

- **WHEN** the `workspace-governance` packaged adapter is not executable in the sibling checkout
- **THEN** the README documents the refusal and the sibling-owned prerequisite rather than promising a workaround