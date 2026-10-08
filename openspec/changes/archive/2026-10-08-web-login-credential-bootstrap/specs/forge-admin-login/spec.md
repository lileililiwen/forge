# forge-admin-login (delta)

## ADDED Requirements

### Requirement: Non-interactive administrator credential input

Forge SHALL accept the administrator password from standard input for `forge
identity setup` and `forge identity change-password` through a
`--password-stdin` flag. With the flag set, Forge SHALL read exactly one line
from standard input, SHALL NOT require a terminal, SHALL NOT prompt, and
SHALL skip the interactive confirmation. Without the flag, the existing
hidden double-entry terminal path SHALL be unchanged. Forge SHALL NOT provide
any flag that accepts the password as an argument value.

#### Scenario: Setup driven from a pipe

- **WHEN** an operator runs `forge identity setup --email <address>
  --password-stdin` with a valid password on standard input while stdin is
  not a terminal
- **THEN** Forge creates the administrator exactly as the interactive path
  does, without prompting and without reading a confirmation

#### Scenario: Password change driven from a pipe

- **WHEN** an administrator exists and the operator runs `forge identity
  change-password --password-stdin` with a valid new password on standard
  input while stdin is not a terminal
- **THEN** Forge replaces the stored Argon2id hash and revokes every
  outstanding session, without prompting

#### Scenario: Empty or missing standard input

- **WHEN** `--password-stdin` is set but standard input is empty or at
  end-of-file
- **THEN** Forge returns a typed error, creates or changes nothing, and
  prints no password

#### Scenario: Short password on standard input

- **WHEN** `--password-stdin` supplies a password shorter than the
  administrator minimum
- **THEN** Forge refuses with the same policy error as the interactive path
  and leaves any existing administrator and sessions unchanged

### Requirement: Administrator configuration status is inspectable

Forge SHALL provide `forge identity status`, a read-only verb that reports
whether the single Forge-wide administrator is configured and, when it is,
the administrator email. It SHALL NOT read or print the stored password hash
or any password, and SHALL NOT create an administrator. The human form SHALL
be trivially parseable as `configured: true` followed by `email: <address>`,
or the single line `configured: false`; the JSON form SHALL carry a pinned
contract, a `configured` boolean and an `email` string or null.

#### Scenario: Status on a fresh registry

- **WHEN** `forge identity status` runs against a registry with no
  administrator
- **THEN** it prints `configured: false` (or JSON `configured: false`,
  `email: null`) and writes no administrator row

#### Scenario: Status on a configured registry

- **WHEN** an administrator exists
- **THEN** `forge identity status` prints the configured state and the
  stored email, and never the password or its hash
