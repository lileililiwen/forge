# github-project-metadata-adapter Specification

## Purpose
An optional, versioned GitHub observation provider that reports explicit provider states, keeps mutation reviewable, and preserves credentials and namespaces.
## Requirements
### Requirement: Versioned, optional GitHub observation

Forge SHALL expose an optional GitHub metadata adapter, contract
`forge-github-metadata/0.1.0`, that observes repository description, topics,
languages, default branch, archived state, workflows, releases, tags and custom
properties, and SHALL normalize each value into the catalog while retaining its
source, revision and freshness.

#### Scenario: GitHub values carry provenance

- **WHEN** a repository is observed through the adapter
- **THEN** each catalog value is recorded with `source = github`, its revision
  and a freshness derived from the observation time

#### Scenario: Local-only Forge keeps working

- **WHEN** no GitHub adapter or token is configured
- **THEN** the catalog and every local command continue to work, and GitHub is
  reported as an unavailable optional source rather than an error

### Requirement: Provider states are explicit

Forge SHALL represent unauthorized, forbidden, not found, rate limited, network
unavailable, stale and partial responses as their own states, and SHALL NOT
present partial or stale data as a complete current observation.

#### Scenario: Rate limiting is not truncated success

- **WHEN** the GitHub API rate limit is reached
- **THEN** the adapter reports `RateLimited` with the reset information and
  returns no partially complete result as if it were whole

#### Scenario: Missing credentials are unauthorized, not failed

- **WHEN** no token is available
- **THEN** the observation state is `Unauthorized` and no request is attempted
  with an empty credential

### Requirement: Reviewable, non-implicit mutation

Forge SHALL default to a pull-request mode for approved metadata changes, SHALL
require an explicit separate mode and confirmation for direct mutation, and
SHALL NOT change repository settings implicitly.

#### Scenario: The default mutation path is a pull request

- **WHEN** an approved metadata change is submitted
- **THEN** the adapter proposes a reviewable pull request in the default mode

#### Scenario: Direct mutation is explicit

- **WHEN** direct mutation is requested without the explicit mode and
  confirmation
- **THEN** Forge refuses and performs no write

### Requirement: Credentials and namespaces are preserved

Forge SHALL keep tokens out of reports and logs, SHALL redact response bodies
before display, and SHALL keep GitHub topics, GitHub release tags and Forge
portfolio tags as separate namespaces.

#### Scenario: A token never reaches a report

- **WHEN** any adapter output is rendered
- **THEN** the token and any credential-shaped value are absent and redacted

#### Scenario: Namespaces do not merge

- **WHEN** GitHub topics are imported
- **THEN** they are retained as GitHub source values and are never presented as
  Forge portfolio tags

