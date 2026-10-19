# Proposal: Replace the single-vendor workspace identity policy

## Why

Accepted decision D2 names Zitadel as the single shared identity provider. That conflicts with the open, provider-neutral direction. Workspace administrators need reusable SSO; product end-user populations must remain isolated. Provider choice and app adoption belong to operators and consuming projects, not a mandatory workspace runtime.

**Implementation authorized (2026-10-19):** the owner directed implementation of
this package. The authorized surface is the governance decision register, its
human view, and this module's own tests. No runtime change, provider
deployment, migration, sibling-repository edit, or publish is authorized.

## What Changes

- Add permanent decision D9 as the provider-neutral successor to D2, and mark D2 superseded without changing its historical text.
- Keep shared workspace SSO internal-only; public static sites remain excluded.
- Define distinct identity populations: workspace administrators may use shared SSO; product end users are project-isolated by default and never receive cross-project access from authentication alone.
- Preserve the current Zitadel deployment as an operator's existing choice until a separately authorized runtime change; this decision does not migrate or remove it.
- Record D9 in `decisions.json`, in `docs/toolchain-decisions.md`, and in the `workspace-decision-register` canonical spec; cover the new lifecycle in `tests/test_decisions.py`.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `provider-neutral-identity-policy` | Set provider-neutral identity policy and population boundaries | `forge/modules/workspace-governance`; Python/JSON decision register | OIDC standard; internal-workspace policy | None | Decision register and canonical requirement agree; public static sites remain excluded |
| `project-oidc-auth-adoption` | Offer opt-in generic OIDC in ASP.NET Core applications while retaining app-local users and permissions | `forge/modules/dotnet-platform-libs`; C#/.NET | OpenID Connect; application-owned users | This policy | Two apps do not share user persistence or authorization; provider-independent OIDC behavior |
| `forge-auth-provider-extensions` | Offer one simple opt-in auth setup journey with replaceable Keycloak and Supabase provider packs | `forge`; Rust CLI + provider module boundary | OIDC; provider-pack extension contract | This policy; .NET package for generated ASP.NET target | Keycloak local trial and optional Supabase configuration work only after explicit adoption |

Dependency order: policy → .NET OIDC integration → Forge provider packs for ASP.NET Core projects. The Forge package may be authored concurrently but must not redefine the policy or .NET contract. Platform-contracts is unchanged because standard OIDC is the wire contract and no portfolio-specific cross-language payload is required. Rust platform libraries remain unchanged: `platform-identity` deliberately excludes OIDC/HTTP host concerns and Forge's existing admin OIDC is not evidence of multiple Rust application consumers. `jenkins-local` remains unchanged: this package does not deploy, replace, or operate workspace identity runtime.

The outcomes span separate owners, languages, release lifecycles, and acceptance oracles; they are not combined into an umbrella implementation package. This package owns policy only.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge admin identity | `forge/openspec/specs/central-admin-identity/spec.md`; `forge identity` | Standard OIDC, PKCE, per-project sessions, explicit admin claims | Forge admin access is not product-user auth or provider lifecycle | Forge repository; separate capability | **adopt** OIDC and isolation principles, not Forge implementation |
| .NET identity platform | `forge/modules/dotnet-platform-libs/openspec/specs/platform-identity-authorization/spec.md` | Provider-neutral identity seam and permission contracts | Application user storage and login UX remain app-owned | Independently adoptable .NET module | **extend shared owner** through its own package |
| Rust platform identity | `forge/modules/rust-platform-libs` identity scope | Provider-neutral claims/authorization boundary | Explicitly excludes OIDC host/network concerns; no multiple Rust consumer evidence | Independently runnable Rust module | **keep local** to consuming Rust apps until recurring consumers justify extraction |
| Existing workspace provider | D2 in `forge/modules/workspace-governance/decisions.json`; current runtime is operator-selected | Shared workspace SSO deployment exists | Current decision fixes the vendor | In-tree governance module owns policy; runtime owner is separate | **adapt through a generic adapter** by policy, without runtime migration |

## User Experience and Interface Impact

No direct screen or CLI changes. Workspace operators use the decision register and existing OIDC runtime. Internal product owners can select a compatible provider; public static-site visitors encounter no new login. Downstream human-facing adoption is owned by the .NET and Forge packages, including accessible login/registration surfaces. No UI is built in this policy package.

## BFS Impact Map

- **Capabilities and actors:** workspace identity policy; workspace administrators; product owners; product end users; public static-site visitors.
- **Contracts and data:** OIDC remains the interoperability protocol. No shared product-user database, cross-project user grants, shared cookies, or new identity payload contract.
- **Configuration and callers:** Forge provider packs and .NET OIDC integration consume this policy; current provider configuration remains unchanged absent opt-in.
- **Failure and security:** provider failure denies login; authenticated status alone grants no admin or product permission; unknown issuer/audience is rejected; no secrets in the register.
- **Verification:** policy audit proves OIDC/provider neutrality, population isolation, internal-only shared SSO, and public-site exclusion; runtime deployment is outside this package.
- **Unaffected:** provider code, user persistence, UI, deployment, migration, and credentials are intentionally untouched.

## Capabilities

- `workspace-decision-register` (modified): records D9, provider neutrality, population boundaries, and the unchanged public-site exclusion.

## Non-goals

- Choosing or installing a new workspace provider; removing Zitadel; changing Jenkins/runtime deployment.
- Defining a proprietary federation protocol or shared user database.
- Adding provider code, application login UI, accounts, roles, or permissions.
- Enabling shared login for public static sites.

## Authorized Files

| File | Change |
|---|---|
| `decisions.json` | D2 status becomes `superseded` with its text, date, `policy_boundary` and downstream list unchanged; new entry D9 is appended. |
| `docs/toolchain-decisions.md` | D2 row shows `superseded`; a D9 row and prose section are added; the register rules state the D9 policy. |
| `tests/test_decisions.py` | The lifecycle test accepts `superseded` entries; D9 policy, D2 history retention and D9 consumer references are asserted. |
| `openspec/specs/workspace-decision-register/spec.md` | Canonical capability gains REG-005 through archive promotion. |
