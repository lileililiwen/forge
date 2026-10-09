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

### Requirement: gh-backed observe fallback when the adapter binary is missing

When no GitHub metadata adapter binary is configured but the user's `gh` CLI is available, Forge SHALL observe repositories through a vendored read-only `gh repo view --json` path and synthesize the same versioned observation shape, SHALL label the envelope `adapter_source=gh-cli-fallback`, and SHALL keep every other adapter behavior unchanged.

#### Scenario: Observe via gh when the adapter is absent

- **WHEN** `forge project github observe <owner/repo>` runs with no adapter binary on `PATH`/`FORGE_GITHUB_BIN` and an executable `gh` on `PATH`/`FORGE_GH_BIN`
- **THEN** Forge runs the bounded `gh repo view <owner/repo> --json nameWithOwner,description,repositoryTopics,defaultBranchRef,isArchived,primaryLanguage` read, returns a `current` observation with topics/description/default-branch/archived/language, and reports `adapter_source=gh-cli-fallback`

#### Scenario: No adapter and no gh stays unavailable

- **WHEN** neither the adapter binary nor `gh` is available
- **THEN** Forge returns the original typed `error[github-adapter-unavailable]` and performs no write

#### Scenario: Fallback never needs the adapter token

- **WHEN** the fallback path runs without `FORGE_GITHUB_TOKEN` set
- **THEN** the observation still succeeds off `gh` auth and no unauthenticated adapter request is attempted

### Requirement: gh-backed direct single-topic propose with confirm echo

When no adapter binary is configured but `gh` is available, Forge SHALL accept a direct-mode single-`topic=` propose with a non-empty `--confirm` token, SHALL apply it via `gh repo edit <owner/repo> --add-topic <topic>`, SHALL verify the confirmation locally without logging its value, and SHALL leave pull-request mode and every other field/mode on the adapter-only path.

#### Scenario: Direct topic propose via gh

- **WHEN** `forge project github propose <owner/repo> --mode direct --confirm <token> --set topic=<value>` runs with no adapter binary and an executable `gh`
- **THEN** Forge runs `gh repo edit <owner/repo> --add-topic <value>`, returns `mode=direct state=current`, and the token value appears in no note, log, or JSON envelope

#### Scenario: Direct propose without confirmation is refused

- **WHEN** direct mode is requested without a non-empty `--confirm` token
- **THEN** Forge refuses with `error[github-invalid]` and performs no `gh` mutation

#### Scenario: PR mode still requires the adapter

- **WHEN** `--mode pull-request` (default) is requested with no adapter binary
- **THEN** Forge returns `error[github-adapter-unavailable]` even when `gh` is installed, and performs no write

#### Scenario: Credentials never reach a report

- **WHEN** any fallback observe/propose output is rendered
- **THEN** the token and any credential-shaped value are absent and redacted

