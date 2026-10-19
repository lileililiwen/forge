## Context

The starter identity module contains application workflows and persistence for token generation, refresh, sessions, password operations, 2FA, and impersonation. The platform currently has `Platform.Identity.Contracts`, `Platform.Identity.AspNetCore`, `Platform.Identity.EntityFrameworkCore`, and authorization seams, but must not become a replacement Identity module.

## Goals / Non-Goals

**Goals:**

- Normalize lifecycle commands, outcomes, opaque identifiers, and security-sensitive failure semantics.
- Let applications adapt ASP.NET Identity, external IdPs, or another store behind the contracts.
- Provide revocation and concurrency semantics suitable for multiple application instances.

**Non-Goals:**

- Own `ApplicationUser`, `ApplicationRole`, claims, password hashes, signing keys, or migrations.
- Decide tenant membership, product permissions, session duration, or impersonation approval policy.
- Implement a complete user-management API.

## Decisions

- Separate framework-neutral contracts from ASP.NET Core adapters and EF Core adapters.
- Represent refresh/session handles as opaque values; implementations store only hashes where persistence is required.
- Return stable result codes for invalid, expired, revoked, reused, and policy-denied operations without revealing whether a credential exists.
- Require an application-provided policy/authorization evaluator for password reset, 2FA, and impersonation decisions.
- Model refresh-token rotation as an atomic consume-and-replace operation to prevent replay across instances.

## Risks / Trade-offs

- [Risk] A generic contract cannot encode every IdP → keep provider details behind application adapters and allow opaque metadata.
- [Risk] Incorrect token storage leaks credentials → require hashed-at-rest guidance and tests that inspect only opaque identifiers.
- [Risk] Impersonation becomes a privilege escalation path → default disabled, require explicit policy and audit hooks.

## Migration Plan

1. Add contracts and deterministic fakes without changing existing identity behavior.
2. Pilot refresh/session lifecycle in one application while retaining its current tables.
3. Add optional adapters only after the contract is validated by a consumer.
4. Migrate workflows incrementally; keep rollback at the application service boundary.
