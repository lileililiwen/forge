## ADDED Requirements

### Requirement: Consent vocabulary

The package SHALL define consent categories (Necessary, Preferences, Analytics, Marketing), a consent record binding subject, granted categories, timestamp, and policy version, and a store contract with an in-memory implementation for tests and single-instance products.

#### Scenario: First visit
- **WHEN** no record exists for a subject
- **THEN** only Necessary is granted and the record reports its policy version as current or stale

#### Scenario: Policy upgrade
- **WHEN** the application bumps its policy version past the record's version
- **THEN** previously granted optional categories evaluate as not granted until re-consent

### Requirement: Policy evaluation

The package SHALL evaluate whether a category is granted for a subject from the stored record and the current policy version, failing closed on missing records, unknown categories, and version mismatch.

#### Scenario: Missing record
- **WHEN** a category is checked with no stored record
- **THEN** the decision is denied except for Necessary

### Requirement: Subject-rights contracts

The package SHALL define export and erasure provider contracts keyed by subject identifier, a registry applications populate, and result models reporting per-provider success, item counts, and failures. The package SHALL NOT define subject data shapes.

#### Scenario: Export across providers
- **WHEN** two providers return documents and one throws
- **THEN** the bundle contains both documents and records the failure without losing the successes

#### Scenario: Erasure is authoritative per provider
- **WHEN** a provider reports erased counts
- **THEN** the report carries those counts unchanged; the registry aggregates, never interprets

### Requirement: Anonymization helpers

The package SHALL provide deterministic email masking and keyed pseudonyms (HMAC-SHA256 with an application-supplied key). Key management stays application-owned.

#### Scenario: Same input
- **WHEN** the same email and key are supplied twice
- **THEN** both calls return the same pseudonym, and the email local part is masked without the key
