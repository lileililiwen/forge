## MODIFIED Requirements

### Requirement: Read-only delivery status

Forge SHALL expose the project's current delivery `phase`, the latest per-verb journal evidence, and the revision-bound confirmation identifier without exposing credentials, provider command lines, or registry bytes. The phase SHALL be derived from the provider's own answer for the verb: a provider that reported a health other than `healthy` SHALL be projected as `degraded`, and a verb whose provider reported no health SHALL keep the existing verdict.

#### Scenario: Status with no journal rows

- **WHEN** the project has never been preflighted
- **THEN** `forge delivery status` and `GET /v1/projects/{id}/delivery` return `phase=draft`, an empty `evidence` list, and the registered source revision

#### Scenario: Status after a partial run

- **WHEN** the journal carries preflight, stage and promote rows for the same revision
- **THEN** the projection returns `phase=healthy` (or `degraded` if the provider's last `verify` reports unhealthy) plus the latest per-verb evidence, sorted by `op_id` and ordered by verb

#### Scenario: A provider that reports itself unhealthy

- **WHEN** the latest promote row's provider reported a health other
  than `healthy`
- **THEN** the projection reports `phase=degraded` and never `healthy`

#### Scenario: Redaction across surfaces

- Each rendered human table, JSON envelope, UI section, and journal `detail` passes `policy::redact_credentials`; no provider command line, no token-shaped value, no `secret_ref` value, and no query string with a credential key ever appears
