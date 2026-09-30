# portfolio-share Specification

## Purpose
An explicit, approval-gated public share record whose deterministic, idempotent publication excludes private data.
## Requirements
### Requirement: Admin can define an explicit public share record
The system SHALL allow an authorized admin to create or update a share record
for a registered project with public metadata and an allowlist of public
surface paths.

#### Scenario: Private project is not listed
- GIVEN a registered project has no approved share record
- WHEN a public manifest is generated
- THEN the project is absent from the manifest

#### Scenario: Admin URL is rejected
- GIVEN a share record includes `/admin/settings`
- WHEN validation runs
- THEN the record is rejected and the reason is persisted

### Requirement: Publication requires exact approval
The system SHALL publish only the exact manifest revision and hash explicitly
approved by an authorized admin.

#### Scenario: Edited record after approval
- GIVEN revision 4 is approved
- WHEN a title or URL changes before publication
- THEN publication is denied until the new revision is approved

### Requirement: Publication is deterministic and idempotent
The system SHALL produce the same canonical bytes and SHA-256 hash for the same
approved records and SHALL return the original result for a repeated operation
key and hash.

#### Scenario: Retry after timeout
- GIVEN the publisher timed out after accepting operation `op-1`
- WHEN `op-1` is retried with the same hash
- THEN the system does not create a second publication

### Requirement: Public output excludes private data
The system SHALL exclude credentials, local paths, internal ports, private notes,
unredacted logs, private surfaces, and raw analytics from every public manifest.

#### Scenario: Secret-like value
- GIVEN a summary contains a credential-shaped value
- WHEN validation runs
- THEN publication is rejected with a safe finding and the value is not echoed

