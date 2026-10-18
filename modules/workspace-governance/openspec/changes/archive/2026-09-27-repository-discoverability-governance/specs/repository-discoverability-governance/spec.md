# Repository Discoverability Governance

## ADDED Requirements

### Requirement: Standard local metadata is auditable

The governance audit MUST report stable findings when an adopted non-inventory
project lacks a root `LICENSE` or a complete `.project.json.discoverability`
object. It MUST validate description, keywords, topics, audience, use cases,
search phrases, homepage, SPDX license, and visibility without inspecting or
printing credentials.

#### Scenario: Missing standard metadata is exposed

- **WHEN** an adopted `product` project has no `LICENSE` and no
  `discoverability` object
- **THEN** the audit emits `LICENSE_MISSING` and
  `DISCOVERABILITY_MISSING` as errors for that project
- **AND** audit mode does not modify the project

#### Scenario: Inventory projects remain lightweight

- **WHEN** an `inventory` project has only a valid `.project.json`
- **THEN** the audit does not require a license or discoverability block

#### Scenario: Invalid metadata is classified precisely

- **WHEN** a project has a discoverability block with an empty description,
  malformed topics, or an unsupported visibility
- **THEN** the audit emits `DISCOVERABILITY_INVALID` with the affected field

### Requirement: Local repair is additive and source-backed

The initializer MUST support explicit local repair of missing license and
discoverability metadata without replacing existing project files or semantic
content. It MUST refuse to invent missing semantic fields.

#### Scenario: Explicit metadata repairs a project

- **WHEN** the operator supplies valid description, keywords, topics, audience,
  use cases, search phrases, homepage, license, and visibility values
- **THEN** the initializer writes only missing local fields/files
- **AND** the resulting declaration passes the local metadata audit

#### Scenario: Existing content is preserved

- **WHEN** a project already has a license, README, or discoverability values
- **THEN** repair leaves those bytes and values unchanged unless an explicit
  replacement mode is supported outside this requirement

#### Scenario: Semantic metadata is incomplete

- **WHEN** the operator omits required semantic values
- **THEN** repair exits non-zero with the exact missing fields and writes no
  partial discoverability block

### Requirement: GitHub metadata is authenticated and verifiable

The GitHub adapter MUST use `gh`, require successful `gh auth status`, preserve
existing remotes/topics, and distinguish local declarations from verified
remote evidence.

#### Scenario: Authentication is unavailable

- **WHEN** `gh auth status` fails
- **THEN** the adapter emits `GITHUB_AUTH_REQUIRED`, exits non-zero, and tells
  the operator to run `gh auth login` manually
- **AND** it does not create a repository, edit metadata, or print credentials

#### Scenario: Existing repository metadata matches

- **WHEN** authenticated `gh repo view` returns the declared owner, name,
  description, homepage, visibility, topics, and branch
- **THEN** `--check` records `metadata_verified` without mutation

#### Scenario: GitHub metadata drifts

- **WHEN** the authenticated repository description or topics differ from the
  local declaration
- **THEN** `--check` emits `GITHUB_METADATA_DRIFT` with the differing fields
- **AND** only explicit `--apply` may run `gh repo edit`

#### Scenario: New repository publication is unambiguous

- **WHEN** auth succeeds, the local repository has an explicit owner/name and
  visibility, and no conflicting origin exists
- **THEN** `--publish` uses `gh repo create --source=. --remote=origin --push`
  without force-pushing
- **AND** a subsequent `gh repo view` verification is required
