# Proposal: Align the .NET platform with shared portfolio contracts

## Why

`dotnet-platform-libs` already owns reusable identity, admin, audit,
web-composition, testing, release, and operational contracts. It now needs a
stable conformance boundary with the language-neutral platform contracts and a
repeatable adoption path for existing .NET business projects.

## What Changes

- Add conformance fixtures for identity subject, permission, tenant, audit,
  Gate result, and release evidence envelopes.
- Keep application-owned users, roles, persistence, migrations, and business
  policies unchanged.
- Add adoption diagnostics that identify missing or incompatible platform
  integration without rewriting consumer code.
- Extend architecture tests to keep production/test package boundaries and
  framework-neutral package direction explicit.
- Document staged adoption for `chinago`, `arivio`, `ploutify`, `fotofy`,
  `smotoox`, `cvunify`, and `stylify`.

## BFS Impact Map

- **Capabilities:** platform contract conformance and consumer adoption.
- **Contracts:** no breaking changes to existing public packages; shared
  schemas become testable compatibility inputs.
- **Consumers:** .NET products opt into packages independently.
- **Security:** identity and admin flows remain application-owned and fail
  closed where platform policy is absent.
- **Release:** package API baselines, SemVer, audit, and evidence remain in
  this repository; product releases remain product-owned.
- **Non-applicable:** Rust and TypeScript consumers are not modified here.

## Capabilities

- `dotnet-platform-contract-conformance`.
- `dotnet-platform-adoption-diagnostics`.

## Non-goals

- No universal user entity, identity database, login UI, dashboard UI, or
  product migration.
- No dependency on the Rust platform repository at build time.
- No automatic package adoption in business projects.
