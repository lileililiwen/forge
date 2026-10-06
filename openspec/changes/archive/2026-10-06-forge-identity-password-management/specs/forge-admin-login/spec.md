# forge-admin-login Specification

## ADDED Requirements

### Requirement: Self-service administrator password change

Forge SHALL provide a non-destructive `forge identity change-password` CLI verb that replaces the single Forge-wide administrator password. It SHALL read the new password twice without terminal echo, apply the same strength and confirmation policy as initial setup, persist only a new Argon2id hash for the existing normalized email, and revoke every outstanding administrator session so no previously issued cookie survives the change. Forge SHALL NOT change the stored email, SHALL refuse the operation when no administrator is configured, and SHALL NOT print the password.

#### Scenario: Operator changes their own password

- **WHEN** the operator runs `forge identity change-password` and enters a valid matching new password twice
- **THEN** Forge replaces only the stored Argon2id hash, keeps the same normalized email, and prints a success message without echoing the password

#### Scenario: Password change invalidates existing sessions

- **WHEN** a password change succeeds while administrator sessions are outstanding
- **THEN** Forge revokes those sessions so a subsequent protected admin API request with a previously issued cookie returns `401`

#### Scenario: Weak or mismatched new password

- **WHEN** the new password is shorter than the minimum length, exceeds the maximum, or the two entries differ
- **THEN** Forge returns a safe CLI error and leaves the stored hash and all sessions unchanged

#### Scenario: Change before any administrator exists

- **WHEN** `forge identity change-password` runs against a registry with no configured administrator
- **THEN** Forge returns a safe error directing the operator to run setup first and writes no administrator row

### Requirement: Strong password generation

Forge SHALL provide `forge identity generate-password` that prints exactly one cryptographically strong random password drawn from operating-system entropy, with a bounded configurable length defaulting to a value that satisfies the administrator password policy. The generator SHALL NOT read or write the registry, SHALL NOT require a terminal, and SHALL NOT persist or log the generated value; it SHALL emit the password to standard output once so an operator can paste it into the change-password prompt.

#### Scenario: Generate a default-strength password

- **WHEN** the operator runs `forge identity generate-password` with no arguments
- **THEN** Forge prints a single random password whose length meets the administrator minimum and does not contact the registry

#### Scenario: Requested length is out of bounds

- **WHEN** the operator requests a length below the administrator minimum or above the supported maximum
- **THEN** Forge returns a safe CLI error and prints no password
