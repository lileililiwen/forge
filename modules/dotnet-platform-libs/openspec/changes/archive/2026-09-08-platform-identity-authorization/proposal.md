# Proposal: Identity and authorization capability

## Why

The portfolio repeats current-user access, password/login flows, external login seams,
permission checks, role checks, and development identity providers. FullStackHero provides a
strong reference, but product roles and user entities must not leak into the shared platform.

## Scope

Add replaceable identity contracts, current-user context, authentication registration seams,
OAuth/OIDC and SMS interfaces, permission catalogs, ASP.NET policy helpers, and deterministic
test providers. Provide an optional EF Core implementation separately.

## Non-goals

- no mandatory user entity, database schema, JWT format, OAuth vendor, SMS vendor, or tenant model;
- no product-specific roles or business permissions;
- no admin screens or subscription workflows; those are separate changes.

## API impact

Adds public identity, authorization, options, policy, and testing contracts. Existing core
packages remain free of ASP.NET Core and EF Core.
