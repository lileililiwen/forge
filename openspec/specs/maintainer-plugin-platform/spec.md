# maintainer-plugin-platform Specification

## Purpose
TBD - created by archiving change maintainer-plugin-platform. Update Purpose after archive.
## Requirements
### Requirement: Registered projects carry compose and CI evidence

Forge SHALL report compose and CI evidence for every
registry-registered project, derived from that project's own
directory, using the detection Forge already performs at adopt
time. A project with a compose file alongside a container file
SHALL report `docker+compose`; a project with only a container
file SHALL report `docker`; a project with neither SHALL report
`none`. A project whose directory cannot be read SHALL report
`none` with its evidence marked unavailable, never a false
`none`.

#### Scenario: Compose predicate returns the projects that ship compose
- **WHEN** an operator runs `forge project list --compose docker+compose`
- **THEN** every registered project whose directory contains
  `docker-compose.yml`, `docker-compose.yaml`, `compose.yaml`
  or `compose.yml` is returned, and no project without one is

#### Scenario: A container file alone is not compose
- **WHEN** a registered project has a `Dockerfile` but no compose file
- **THEN** its `compose` field is `docker`, and it is not
  returned by `--compose docker+compose`

#### Scenario: An unreadable project is reported honestly
- **WHEN** a registered project's directory cannot be read
- **THEN** its `compose` field is `none` and its evidence is
  marked unavailable; Forge does not report a compose state it
  could not observe

### Requirement: Classification is derivable from repository evidence

Forge SHALL derive a project's profile, lifecycle, portfolio tags
and domain from evidence already present in the repository or
already observed from it — the manifest's declared `stack`,
`profile` and `maturity`, the GitHub `topics` and `language` the
GitHub adapter already observes, and the README's first heading —
and SHALL record that evidence with each proposal. Deriving SHALL
run no model and no network call, and SHALL write no field on any
project or provider.

#### Scenario: Deriving from a declared manifest
- **WHEN** an operator runs `forge classify derive` on a project
  whose manifest declares a profile and a maturity
- **THEN** a `profile` and a `lifecycle` proposal are recorded
  at high confidence, each naming the manifest field it was
  derived from, and no project field is changed

#### Scenario: Deriving tags from an observed remote
- **WHEN** a project's GitHub topics have been observed
- **THEN** a `portfolio-tags` proposal records them at medium
  confidence, keeping GitHub topics, GitHub release tags and
  Forge portfolio tags in three separate namespaces

#### Scenario: Deriving is deterministic
- **WHEN** `forge classify derive` runs twice on an unchanged
  project with no network observation
- **THEN** both runs record the same proposals at the same
  confidence

### Requirement: Approved metadata is published through a named plugin

Forge SHALL route approved project metadata outward through a
configured plugin rather than mutating any remote itself. Forge
SHALL refuse to apply a proposal that is not yet approved, and
SHALL refuse any approved value outside the fields the metadata
adapter already permits. The remote change SHALL be proposed
through a reviewable pull request.

#### Scenario: An unapproved proposal is refused by name
- **WHEN** an operator runs `forge classify apply` while a
  proposal is still in the `Suggested` state
- **THEN** Forge refuses and names that proposal; nothing is
  sent to any plugin

#### Scenario: Approved metadata reaches the metadata plugin
- **WHEN** an operator applies a set of approved proposals and a
  plugin advertises the metadata capability
- **THEN** Forge sends one `forge-metadata-propose/0.1.0`
  request in pull-request mode, and records a journal row
  naming the plugin, the fields, and the returned pull-request
  reference

#### Scenario: No metadata plugin is a refusal, not a no-op
- **WHEN** an operator applies approved metadata and no
  configured plugin advertises the metadata capability
- **THEN** Forge refuses and names the missing capability; it
  does not silently discard the approved values

#### Scenario: Forge never mutates a remote directly
- **WHEN** any metadata is published
- **THEN** it reaches the remote only through a plugin, in
  pull-request mode; no direct mutation path is reachable

### Requirement: Plugins are named, enumerable and capability-declared

Forge SHALL treat every configured external integration — GitHub,
OpenPanel, and any future remote — as a named plugin with a
declared kind and a declared capability list, and SHALL list them
on demand. A plugin with no descriptor SHALL be treated as a
delivery plugin, so existing configuration keeps working
unchanged. A plugin that declares an unknown capability SHALL be
reported as invalid rather than silently accepted.

#### Scenario: Listing the configured plugins
- **WHEN** an operator runs `forge plugins`
- **THEN** every configured plugin is listed with its id, kind,
  enabled state and declared capabilities

#### Scenario: An existing configuration needs no edit
- **WHEN** a `providers.yaml` entry carries only `id`, `command`
  and `enabled`
- **THEN** it is reported as a delivery plugin and keeps working
  exactly as before

#### Scenario: An unknown capability is refused
- **WHEN** a plugin descriptor declares a capability outside the
  closed set
- **THEN** the plugin is reported invalid and the registry does
  not advertise that capability

#### Scenario: A missing plugin command does not hide the others
- **WHEN** one configured plugin's command is not on the path
- **THEN** that plugin is reported unavailable with its reason,
  and the remaining plugins are still listed

### Requirement: The browser exposes the maintainer surface

Forge SHALL let an operator search the fleet by the catalog's
existing predicates and maintain one project's metadata without
leaving the browser. The browser SHALL render what Forge last
observed, its freshness, the derived proposals, and the publish
action.

#### Scenario: Filtering the fleet
- **WHEN** an operator filters the fleet by language, lifecycle,
  profile, compose, CI or tag
- **THEN** the table shows the same rows the equivalent
  `forge project list` invocation returns for the same predicate,
  and the filter composes with the free-text search

#### Scenario: Maintaining one project
- **WHEN** an operator opens a project's Maintain card
- **THEN** the card shows the observed GitHub description,
  topics, homepage and language with their freshness, the derived
  proposals with per-field approve and reject, and one action that
  applies the approved set through the plugins

#### Scenario: An unreachable remote is stated, not blank
- **WHEN** a project's GitHub remote cannot be observed
- **THEN** the card reports the remote as unavailable with the
  reason, rather than showing empty fields that read as "no
  description set"

