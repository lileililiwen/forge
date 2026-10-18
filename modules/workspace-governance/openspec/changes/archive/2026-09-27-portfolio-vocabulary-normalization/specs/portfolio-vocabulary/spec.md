# portfolio-vocabulary Specification

## Purpose

Make `workspace-governance` the single source of the portfolio vocabulary —
canonical kind and profile values plus the shared placeholder-marker and
secret-field word lists — and normalize every existing declaration to it.

## ADDED Requirements

### Requirement: One machine-readable vocabulary source

`vocabulary.json` MUST define the canonical `profiles`, `kinds`,
`placeholder_markers` and `secret_field_substrings`. A missing, unparseable,
duplicate-bearing or empty-list vocabulary MUST make a consumer exit non-zero
with the reason and MUST NOT fall back to an empty list.

#### Scenario: Vocabulary loaded

- **WHEN** a consumer starts and `vocabulary.json` is valid
- **THEN** it uses the canonical sets without a local duplicate list

#### Scenario: Vocabulary unusable

- **WHEN** `vocabulary.json` is missing or lists an empty
  `secret_field_substrings`
- **THEN** the consumer exits non-zero and names the file and the problem

### Requirement: Canonical profiles are enforced

A registry entry whose `profile` is outside the canonical set MUST be ERROR
`PROFILE_UNKNOWN` naming the value, and the ad-hoc allow-list MUST be removed. A
`profiles/<name>.json` document naming a non-canonical profile MUST be ERROR
`PROFILE_DOC_UNKNOWN`.

#### Scenario: Canonical profile

- **WHEN** a registry entry uses a canonical profile
- **THEN** no profile finding is reported

#### Scenario: Non-canonical profile

- **WHEN** a registry entry declares a one-off profile such as `web-saas`
- **THEN** the audit reports ERROR `PROFILE_UNKNOWN` naming the value

#### Scenario: Profile document disagrees

- **WHEN** `profiles/` contains a document for a profile outside the set
- **THEN** the audit reports ERROR `PROFILE_DOC_UNKNOWN`

### Requirement: Canonical kinds are enforced

A declaration whose `kind` is outside the canonical set MUST be WARNING
`KIND_UNKNOWN` naming the value, and `kind` MUST NOT drive profile
requirements.

#### Scenario: Non-canonical kind

- **WHEN** a declaration uses a one-off kind such as `quality-gate`
- **THEN** the audit reports WARNING `KIND_UNKNOWN` and the project's required
  files are unchanged

### Requirement: Shared word lists have one source

The secret scan MUST use `secret_field_substrings` from `vocabulary.json`, and
the product-code quality checker MUST default its placeholder markers from
`placeholder_markers`. A per-project `placeholder_markers` override MUST still
be honoured. The secret list MUST remain a superset of the previous terms.

#### Scenario: Marker default from vocabulary

- **WHEN** a project declares no `placeholder_markers`
- **THEN** the checker uses the vocabulary's list

#### Scenario: Override still honoured

- **WHEN** a project declares its own `placeholder_markers`
- **THEN** the checker uses those instead

#### Scenario: Secret scan still fires

- **WHEN** a registry entry contains a value under a secret-shaped key named in
  the vocabulary
- **THEN** the audit reports ERROR `SECRET_VALUE`

### Requirement: The portfolio is normalized

Every registry entry and declaration MUST use a canonical `profile` and `kind`.

#### Scenario: Scan is clean

- **WHEN** the canonical sets are compared against every project
- **THEN** no non-canonical value remains
