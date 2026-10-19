# Design: Rust platform foundation

## Crate boundaries

`platform-core` contains bounded results, safe error categories, opaque IDs,
clock traits, correlation IDs, and redaction helpers. `platform-contracts`
contains serialized representations matching `platform-contracts` schemas.
`platform-identity` contains current-subject, credential, session, permission,
tenant, and lifecycle traits. `platform-audit` contains normalized audit events
and sinks. `platform-testing` contains deterministic fakes and builders.

Production crates MUST NOT depend on test support. Core crates MUST NOT depend
on Axum, SQLx, a database driver, HTTP client, provider SDK, or product crate.

## Identity boundary

The application owns user records, password hashing policy, session storage,
token transport, provider configuration, and migrations. The platform exposes
traits and safe normalized outcomes only. Implementations may later be added as
separate adapters after two consumers demonstrate the same semantics.

## Serialization and compatibility

Serde representations carry the namespaced contract/version envelope and use
explicit enums for status and failure categories. Unknown additive fields are
ignored where the shared schema permits; unsupported major versions and
missing required fields fail without fabricated defaults.

## Verification

Tests run without network, databases, or external providers. Fixtures are
compared against the language-neutral contract repository. Architecture tests
assert dependency direction and ensure test-only crates cannot be referenced by
production crates.
