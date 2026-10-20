# hosted-ci-evidence-envelope Specification

## Purpose
Hosted CI runs produced results that Forge could not accept or even describe,
because nothing said what a hosted observation must carry. The versioned
`forge-hosted-ci-evidence/1.0` envelope binds a project, repository, workflow,
run, attempt, job, revision, timestamp and one existing Gate evidence record,
and validation refuses unknown fields, unsupported contracts, rehearsal or
non-full evidence, mismatched identities and unsafe URLs. A document that passes
validation still carries producer claims, not authentication.
## Requirements
### Requirement: Forge-owned hosted CI evidence envelope

Forge SHALL define and validate the versioned `forge-hosted-ci-evidence/1.0` envelope that binds a Forge project, repository slug, GitHub Actions workflow/run/attempt/job identity, source revision, observation timestamp, and one existing Forge Gate evidence record. Validation SHALL accept supported Gate evidence contracts `0.1.0` and `0.2.0`, interpret legacy `0.1.0` records without profile metadata as `full`, and reject any evidence whose profile is not `full`. Validation SHALL reject unknown envelope/run fields, unsupported contract versions, malformed or inconsistent identities, malformed or mismatched revisions, rehearsal evidence, unknown aggregate results, invalid timestamps, unsafe run URLs, and credential-shaped embedded diagnostics. The envelope SHALL NOT accept environment values, raw logs, or arbitrary metadata.

#### Scenario: Consistent hosted run envelope validates

- **WHEN** a document has supported contracts, a valid project and repository, consistent GitHub run metadata, a valid HTTPS GitHub run URL, a full source SHA matching real full-profile Gate evidence, and a valid timestamp
- **THEN** Forge parses and serializes the typed envelope without changing the embedded Gate evidence semantics

#### Scenario: Legacy full evidence remains compatible

- **WHEN** a valid legacy `0.1.0` Gate record has no `profile` field
- **THEN** Forge interprets it as full-profile evidence and accepts it subject to all other envelope checks

#### Scenario: Fast evidence is refused as hosted verification

- **WHEN** an otherwise valid envelope contains a real `fast` Gate record
- **THEN** validation refuses it and no hosted observation is persisted

#### Scenario: Producer claims remain untrusted

- **WHEN** the envelope validator accepts a well-formed document
- **THEN** Forge treats repository, workflow identity, timestamp and aggregate as producer-supplied claims only and SHALL NOT label them authenticated or use them as delivery approval

#### Scenario: Identity or revision contradiction is refused

- **WHEN** project identity is invalid, Gate project differs, source revision is malformed, or embedded Gate revision differs from the envelope revision
- **THEN** validation fails with a bounded field-specific error and no evidence is persisted

#### Scenario: Rehearsal or non-verdict evidence is refused

- **WHEN** the embedded Gate evidence is dry-run, unbound, or has an unknown aggregate
- **THEN** validation refuses it as hosted run evidence

#### Scenario: Untrusted URL or extra payload is refused

- **WHEN** the run URL uses a non-GitHub host, credentials, query, fragment, or path that does not match the repository/run id, or the document contains an unknown field
- **THEN** validation fails without echoing the submitted document

#### Scenario: Format validity is not authentication

- **WHEN** a future consumer receives an envelope that passes local schema validation
- **THEN** it still requires authenticated caller identity and registered repository/project binding before accepting or exposing a hosted observation
