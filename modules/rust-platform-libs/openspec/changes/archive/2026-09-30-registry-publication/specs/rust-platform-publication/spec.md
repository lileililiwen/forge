# rust-platform-publication Specification

## ADDED Requirements

### Requirement: Every publishable platform crate SHALL carry complete registry metadata (PUB-001)

All production crates except `platform-arch-tests` MUST be publishable with
`repository`, `homepage`, `documentation`, per-crate `description`,
`keywords`, `categories`, license, and exclusion of repository-internal
paths from the packaged archive, and MUST pass `cargo publish --dry-run`
with registry-addressable inter-crate dependencies.

#### Scenario: Dry-run packaging succeeds for the DAG

- **WHEN** each publishable crate is dry-run in topological order
- **THEN** packaging succeeds, the package list excludes `openspec/` and
  `.github/`, and dependency versions resolve from the registry form

#### Scenario: Internal tooling refused

- **WHEN** publication is attempted for `platform-arch-tests`
- **THEN** the manifest refuses via `publish = false`

### Requirement: Publication SHALL happen only from a version-matching tag, in order, idempotently (PUB-002)

The tag-triggered workflow MUST refuse when the tag does not equal the
workspace version or the approval marker or registry token is absent, MUST
publish crates in dependency order, MUST skip crates whose exact version
already exists, and MUST confirm registry propagation per crate.

#### Scenario: Ordered release

- **WHEN** tag `v0.1.0` runs with all preconditions met
- **THEN** `platform-core` publishes first and dependent crates publish
  only after their prerequisites confirm, each with a propagation poll

#### Scenario: Rerun after partial failure

- **WHEN** the workflow reruns with some crates of the version already
  published
- **THEN** those crates are skipped, the remainder publish, and no
  version is republished

#### Scenario: Version mismatch refuses everything

- **WHEN** the tag semver differs from the workspace package version
- **THEN** the job fails naming both values before any publish step runs

### Requirement: Version, yank, and token policy SHALL be documented and binding (PUB-003)

`docs/publication.md` MUST define the workspace-lockstep version rule, the
pre-1.0 breaking policy, the immutable-version yank procedure with recorded
reason and replacement, and crate-scoped token handling; releases without a
matching changelog entry are non-conformant.

#### Scenario: Breaking pre-1.0 bump is legal only as a minor

- **WHEN** a breaking change ships while the workspace version is `0.y.z`
- **THEN** the version bumps `0.(y+1).0` and the changelog names the break

#### Scenario: Yank is recorded

- **WHEN** a published version is yanked
- **THEN** the policy document records the reason and the replacement
  version in the changelog section

### Requirement: Release SHALL stay gated on explicit owner approval (PUB-004)

No real publication (tag execution) may occur while the GitHub-publication
precondition is unmet or the tracked approval marker file is absent;
implementing the pipeline is allowed, and the change's completion record
MUST keep the release step visibly blocked with the exact external actions
required.

#### Scenario: Missing marker refuses the release

- **WHEN** the publish workflow starts without the approval marker tracked
  in the repository
- **THEN** it fails with the precondition message and zero registry calls
