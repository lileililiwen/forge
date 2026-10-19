# Proposal: Establish the Rust platform foundation

## Why

Several Rust web and operator projects independently implement identity,
sessions, audit, errors, and test fakes. A large web framework would couple
them prematurely, but pure contracts and deterministic support can reduce
security and compatibility drift.

## What Changes

- Create a Cargo workspace with small, dependency-light foundation crates.
- Implement Rust representations of the language-neutral platform contracts.
- Define provider-neutral identity, permission, tenant, audit, error, and
  correlation interfaces.
- Add deterministic test-support boundaries.
- Prohibit Axum, SQLx, cookie, OIDC, JWT, and product database dependencies in
  the foundation crates.

## BFS Impact Map

- **Capabilities:** Rust contract types, serialization, safe outcomes, and
  deterministic test support.
- **Consumers:** openaccounting, rust-ecommerce, hookit, openpanel, vowra, and
  future Rust services may adopt selectively.
- **Persistence:** none; applications own tables, migrations, and stores.
- **Security:** no credential or token values in errors; identity failures use
  stable safe categories.
- **Compatibility:** contract identifiers and fixture parity come from
  `platform-contracts`; Rust APIs remain semver-governed.
- **Tests:** unit tests, serde fixtures, property/boundary tests, and public
  API/dependency direction checks.

## Capabilities

- `rust-platform-foundation`: Rust products can consume shared contracts
  without adopting a web framework or database.

## Non-goals

- No login/register routes, dashboards, JWT, cookies, OIDC, SQLx, Axum, or
  application entity types.
- No migration of existing products in this foundation change.
- No claim of production security from type or fixture tests alone.
