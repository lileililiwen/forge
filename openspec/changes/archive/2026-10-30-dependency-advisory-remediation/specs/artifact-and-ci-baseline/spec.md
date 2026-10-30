## ADDED Requirements

### Requirement: Advisory exceptions are reachable, reasoned and agreed

Forge SHALL upgrade a dependency advisory that has a compatible fixed release.
When no fixed release exists, Forge SHALL record an exception that names the
reachable code path, why the affected operation cannot be exercised, and the
upstream trigger that ends the exception. Every vulnerability-class exception
SHALL be present in both dependency-policy files.

#### Scenario: A fixed release exists

- **WHEN** an advisory names a compatible fixed version
- **THEN** the dependency is upgraded and no exception is recorded for it

#### Scenario: No fixed release, unreachable path

- **WHEN** the only affected operation needs a private key or a signing
  operation that Forge never performs
- **THEN** the exception names that boundary and the transitive path, and both
  `deny.toml` and `.cargo/audit.toml` carry it

#### Scenario: One policy file is edited

- **WHEN** a vulnerability-class exception exists in one policy file but not
  the other
- **THEN** the policy contract test fails naming the advisory and both files

#### Scenario: The exception ends

- **WHEN** an upstream release removes the affected code path
- **THEN** the exception is removed and the upgrade is recorded, rather than
  the reason being left to rot
