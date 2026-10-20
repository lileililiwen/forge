# project-changelog-bootstrap Specification

## Purpose

Every project Forge scaffolds starts with a changelog, so the release lane's
requirement for one is satisfied by the scaffold rather than by a file the
owner has to write before the first release can be prepared. The generated
file claims no release and carries no date: a new project has not been
released, and the scaffold's assembly is deterministic. The file lands at the
path the release lane reads by default, and every other generated file is
unchanged.

## Requirements
### Requirement: A scaffolded project carries a changelog

Forge SHALL write a `CHANGELOG.md` at the root of every generated project, in
keep-a-changelog form, with an `## [Unreleased]` section that names the
scaffold and the profile. The file SHALL land at the path the release lane
reads by default, so a scaffolded project can prepare a release without its
owner writing a changelog first.

#### Scenario: A generated project carries a changelog

- **WHEN** `forge new` creates a project from any supported profile
- **THEN** the tree contains `CHANGELOG.md` with an `## [Unreleased]` section, and every other generated file is unchanged

#### Scenario: A scaffolded project can prepare a release

- **WHEN** an owner runs `forge release prepare` on a project the scaffold created
- **THEN** the plan captures the generated changelog instead of refusing for a missing one

### Requirement: The changelog claims no release and no clock

Forge SHALL write the generated changelog without a released version heading,
without a date, and without any other statement that a release has happened,
because a scaffolded project has not been released. The same creation request
SHALL render the same changelog bytes.

#### Scenario: No release is claimed

- **WHEN** the generated changelog is read
- **THEN** it names no released version and no date, and its only version heading is `## [Unreleased]`

#### Scenario: The assembly stays deterministic

- **WHEN** the same creation request runs twice
- **THEN** the two changelog files are byte-identical

