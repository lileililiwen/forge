# hosted-ci-evidence-ingest Specification

## Purpose

Authenticate GitHub Actions Gate evidence, bind it to a registered Forge project and immutable repository identity, retain it idempotently, and make it reviewable in the Forge project workbench.

## ADDED Requirements

### Requirement: Authenticated GitHub Actions ingest

Forge SHALL accept `forge-hosted-ci-evidence/1.0` only through an OIDC-authenticated workload route that validates a GitHub Actions token signature, fixed issuer, fixed audience, validity window, required claims, and exact equality between signed repository/run/workflow/revision identity and the submitted envelope. Only branch `push` observations from the generated `.github/workflows/forge-gate.yml` workflow are eligible. The signed repository slug and numeric repository ID SHALL match a registered project remote and its operator-bound immutable repository ID. The request SHALL be bounded, and no evidence SHALL be persisted before all checks pass.

#### Scenario: Valid signed run is accepted
- **WHEN** a valid GitHub Actions ID token and matching terminal Gate envelope identify a registered, immutably bound project
- **THEN** Forge stores the observation and returns its identity without returning or persisting the token

#### Scenario: Invalid identity fails closed
- **WHEN** signature, issuer, audience, time, required claim, envelope claim, registered slug, or repository ID does not match
- **THEN** Forge refuses the request without a registry write or token disclosure

#### Scenario: Unbound legacy project cannot ingest
- **WHEN** a valid workflow token targets a registered project without an operator-bound repository ID
- **THEN** Forge rejects ingest and identifies the admin binding action without trusting the caller to establish the binding

### Requirement: Immutable project repository binding

Forge SHALL let an authenticated project operator bind one positive decimal GitHub repository ID when the project's registered remote resolves to exactly one canonical `github.com/owner/name` slug. Binding SHALL be immutable through this capability; reasserting the same ID is idempotent, and a different ID conflicts. Existing projects SHALL remain unbound after migration.

#### Scenario: Operator binds the matching repository identity
- **WHEN** an operator confirms a valid repository ID for a project whose remote has one canonical GitHub slug
- **THEN** Forge records the ID and shows the project as ready for hosted evidence

#### Scenario: Existing binding cannot be replaced
- **WHEN** a request tries to bind a different numeric repository ID to an already-bound project
- **THEN** Forge returns a conflict and preserves the stored identity

#### Scenario: Non-GitHub or ambiguous remotes are refused
- **WHEN** the registered remote is absent, non-GitHub, or cannot be normalized to one owner/name slug
- **THEN** Forge refuses binding without modifying the registry

### Requirement: Append-only idempotent hosted observations

Forge SHALL persist each accepted hosted observation append-only, uniquely identified by project, run ID and attempt. The job label is descriptive metadata because it is not a signed OIDC claim. An exact replay SHALL return the existing observation; a replay with changed envelope content for the same identity SHALL return conflict. Project history SHALL be bounded and ordered newest first. Hosted evidence SHALL remain separate from local Gate evidence and delivery preconditions.

#### Scenario: Exact retry is idempotent
- **WHEN** the same authenticated envelope is submitted again for the same run/attempt
- **THEN** Forge returns the existing observation and creates no additional row

#### Scenario: Conflicting replay is refused
- **WHEN** a different digest is submitted for an already-recorded run/attempt identity
- **THEN** Forge returns conflict and preserves the original record

#### Scenario: Hosted pass is not local delivery approval
- **WHEN** hosted history contains a passing record while local Gate evidence is absent or stale
- **THEN** the local Gate and delivery surfaces continue to report their existing local-evidence state

### Requirement: Hosted evidence workbench

The authenticated project Quality checks card SHALL show repository binding state and a bounded hosted observation history alongside, but distinct from, local Gate evidence. It SHALL show workflow/job, run link, run attempt, revision, observed time, aggregate and freshness, with explicit loading, empty, stale, unavailable, permission and success states. It SHALL support keyboard operation, accessible status announcements, visible focus and a narrow viewport without horizontal overflow.

#### Scenario: Operator reviews hosted history
- **WHEN** the project has hosted observations
- **THEN** the workbench shows newest-first entries and freshness against the registered project revision without changing local Gate status

#### Scenario: No hosted observations are present
- **WHEN** the project has no hosted records
- **THEN** the workbench shows an explicit empty state and whether immutable repository binding is required

#### Scenario: Hosted history is unavailable
- **WHEN** the API cannot load hosted history or the operator session has expired
- **THEN** the workbench reports unavailable or requests sign-in and preserves the distinct local Gate card state

### Requirement: Standard workflow evidence upload

New service and client standard snapshots SHALL split verification from upload so only the dependent upload job receives `id-token: write`. The upload job SHALL use the Forge evidence uploader and request the fixed audience. It SHALL run for completed branch-push Gate observations, including a terminal blocked Gate result, so failures are observable while the verification job remains failed. It SHALL run only when a Gate evidence artifact and Forge evidence endpoint are present. It SHALL skip with a visible explanation when no Forge evidence endpoint is configured or the event is a pull request; in these cases no OIDC upload job SHALL run. A configured upload SHALL fail on an error. Historical standard snapshots SHALL remain unchanged.

#### Scenario: Configured trusted push uploads terminal evidence
- **WHEN** the Forge Gate completed on a branch push and the endpoint is configured
- **THEN** the least-privilege upload job requests an audience-bound token and uploads the matching Forge Gate observation

#### Scenario: Endpoint is not configured
- **WHEN** a branch-push workflow runs without a Forge endpoint
- **THEN** verification runs and visibly reports that hosted evidence upload is not configured without starting the OIDC upload job

#### Scenario: Gate blocks or an earlier check prevents evidence
- **WHEN** Gate produces a terminal blocked record, or verification fails before a Gate record exists
- **THEN** the upload job records the blocked Gate result when present, while a missing record keeps the workflow failed without inventing an observation

#### Scenario: Pull request does not receive an ID token
- **WHEN** a pull request runs the verification workflow
- **THEN** the workflow displays the upload skip reason and no job requests an OIDC token

### Requirement: Forge standard evidence uploader

The pinned standard uploader script SHALL require GitHub Actions runtime credentials, a configured HTTPS Forge ingest endpoint, the current supported envelope and a real terminal Gate record for the current commit. It SHALL obtain an ID token using the fixed audience, send only the validated envelope, retry only transient network/429/5xx failures within a fixed bound, and never print or persist tokens.

#### Scenario: Uploader submits real current evidence
- **WHEN** Actions environment, endpoint, project metadata, commit and real Gate evidence agree
- **THEN** the uploader obtains a token and submits a valid envelope, reporting only the bounded result identity

#### Scenario: Missing setup or invalid evidence is refused
- **WHEN** required environment, endpoint, project metadata, current revision, or terminal Gate evidence is absent or invalid
- **THEN** the command exits nonzero before upload and reports a recovery action without exposing credentials

#### Scenario: Forge is transiently unavailable
- **WHEN** the ingest endpoint returns a transient network, 429 or 5xx failure
- **THEN** the uploader retries within its configured bound and exits nonzero with a safe diagnostic if no attempt succeeds
