# Design

Packages:

- `Platform.Identity.Contracts`: current user, external identity, credential, verification,
  session, and provider status records.
- `Platform.Identity.AspNetCore`: authentication scheme wiring, claims projection, policy
  helpers, and middleware/accessor integration.
- `Platform.Identity.EntityFrameworkCore`: optional default stores and schema-independent
  persistence services.
- `Platform.Authorization`: permission definition, catalog, policy naming, and authorization
  decision contracts.
- `Platform.Identity.Testing`: fake identity, current-user, OAuth, and SMS providers.

Permissions use `resource.action` keys and are registered by consuming modules. Roles map to
permissions in the application store. The platform exposes `RequirePlatformPermission` and
permission discovery; it does not own business role names.

OAuth/OIDC, email, and SMS providers return normalized results. Failed authentication must be
classified without leaking provider responses or secrets. All security-sensitive mutations emit
an audit hook consumed by the later admin/audit capability.
