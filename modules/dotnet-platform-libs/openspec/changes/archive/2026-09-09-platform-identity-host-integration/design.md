## Context

Platform identity currently offers normalized credentials, external identities, sessions, verification, current-user contracts, claim projection, and permission authorization. The starter kit owns JWT issuance/refresh, ASP.NET Identity entities, current-user middleware, roles/permissions, sessions, and tenant claim conventions.

Starter-kit references:

- `dotnet-starter-kit/src/Modules/Identity/`
- `dotnet-starter-kit/src/BuildingBlocks/Web/Auth/CurrentUserMiddleware.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Shared/Identity/Claims/ClaimsPrincipalExtensions.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Shared/Identity/Authorization/`
- `dotnet-starter-kit/src/Tests/Identity.Tests/`

Use these to identify behavior and tests, but do not copy starter entity types, JWT keys, claim names, or permission catalogs into the platform.

## Goals / Non-Goals

**Goals:**

- Project authenticated claims into `CurrentUser` consistently.
- Provide permission policy helpers and auditable authorization decisions.
- Define replaceable authentication, credential, session, verification, and persistence seams.
- Validate security-sensitive configuration on startup without logging secrets.

**Non-Goals:**

- Owning ASP.NET Identity user/role entities, JWT issuance, refresh-token storage, password rules, tenant provisioning, or product permissions.
- Choosing cookies vs JWT vs external providers.

## Decisions

1. Keep provider-neutral contracts in existing identity packages and add a host adapter package for claims/authorization.
2. Make claim types, authentication scheme, token issuer, and permission source configurable or application-provided.
3. Expose policy builders and authorization audit hooks instead of static permission constants.
4. Keep EF persistence adapter generic around application-owned user/session entities and stores.

Alternative rejected: importing the starter identity module would impose its schema, token policy, and product permissions on all applications.

## Risks / Trade-offs

- [Risk] Claim mapping mismatch causes authorization failures → require explicit mapping tests and diagnostics without token values.
- [Risk] Insecure default token configuration → validate required issuer/audience/signing settings when the application selects JWT.
- [Risk] Identity persistence differs across products → keep stores replaceable and avoid platform-owned migrations.

## Migration Plan

Register current-user projection first, then permission policies, then replace individual starter services. Keep application authentication issuance and entities unchanged during the pilot. Rollback removes host adapter registration.

## Open Questions

- Whether the EF adapter should support ASP.NET Identity directly or remain limited to platform store interfaces in the first release.
