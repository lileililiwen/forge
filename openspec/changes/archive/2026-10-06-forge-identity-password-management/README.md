# forge-identity-password-management

Adds self-service rotation of the single Forge-wide administrator password
(`forge identity change-password`, which also revokes outstanding sessions) and
an OS-entropy password generator (`forge identity generate-password`). Extends
the `forge-admin-login` capability. No schema change, no new dependency.
