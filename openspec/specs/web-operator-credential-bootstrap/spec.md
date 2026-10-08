# web-operator-credential-bootstrap Specification

## Purpose
Bootstrap the operator's Forge login from `scripts/web.sh`: ensure a Forge administrator exists, print the login URL, account email and a one-time generated password on standard output, and never persist, recover or leak the password.
## Requirements
### Requirement: `start` ensures and prints a usable operator login

`scripts/web.sh start` SHALL ensure a Forge administrator exists before the
services bind, and SHALL print on standard output the login URL
(`http://<web-host>:<web-port>/`), the account email, and — only when it just
created the account with a password it generated — that password exactly
once. It SHALL accept `--admin-email <address>` (default
`operator@example.com`). When an administrator already exists, `start` SHALL
print the email and the login URL and SHALL direct the operator to
`scripts/web.sh reset-password` for a new password, and SHALL NOT print,
invent or reset a password. `start` SHALL never silently reset an existing
account.

#### Scenario: Fresh registry, first start

- **WHEN** `scripts/web.sh start` runs against a registry with no
  administrator
- **THEN** it creates the administrator with a freshly generated password,
  starts the services, and prints the login URL, the account email and the
  password once

#### Scenario: Existing administrator, later start

- **WHEN** `scripts/web.sh start` runs and an administrator already exists
- **THEN** it prints the login URL and the stored email, prints no password,
  and names `scripts/web.sh reset-password` as the way to get a new one

### Requirement: `reset-password` rotates and prints the login once

`scripts/web.sh reset-password` SHALL generate a fresh password, set it
non-interactively through `forge identity change-password --password-stdin`
when an administrator exists (or `forge identity setup --password-stdin` when
none does), honouring `--admin-email`, and SHALL print the login URL, the
account email and the password exactly once. It SHALL state that the
rotation revokes existing sessions. It SHALL refuse with a non-zero exit when
the Forge binary is missing.

#### Scenario: Reset rotates an existing login

- **WHEN** `scripts/web.sh reset-password` runs while an administrator exists
- **THEN** the stored hash is replaced and every previously issued session is
  revoked, and the command prints the email and the new password once

#### Scenario: Reset against a fresh registry creates the login

- **WHEN** `scripts/web.sh reset-password` runs with no administrator
- **THEN** it creates the administrator from the generated password and
  prints the email and password once

### Requirement: The generated password never persists on the operator's machine

`scripts/web.sh` SHALL write the generated password only to standard output.
It SHALL NOT place the password in process arguments, in `.forge/run/state`,
in any service log, or in any other file it writes. The state file SHALL
continue to carry only the listener hosts, ports and projects root.

#### Scenario: Password is stdout-only

- **WHEN** `scripts/web.sh reset-password` runs
- **THEN** the password appears on standard output and nowhere in
  `.forge/run/state` or the service logs

#### Scenario: No password in arguments

- **WHEN** the script invokes `forge identity setup` or `forge identity
  change-password`
- **THEN** the password is supplied on standard input and never appears in
  the invoked command's arguments
