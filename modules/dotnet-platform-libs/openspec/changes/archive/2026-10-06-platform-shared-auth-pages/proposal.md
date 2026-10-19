# Proposal: Shared auth page components, theme switching, and branding metadata

## Why

The platform ships a token layer (`ui/tokens/tokens.json`) and one primitive package
(`ui/packages/react-ui`), but it ships nothing for the pages an application shows first to
an unauthenticated visitor. Every new product rebuilds sign-in, registration, password
recovery, and the theme toggle, and every one of them hand-rolls its inputs, its colors,
and its focus handling. The result is a second visual contract that drifts from the tokens
it claims to use.

Two claims in the repository are also currently false. The `EnableSiteUsers` template
symbol documents "an opt-in self-registration toggle", and archived task 2.4 is checked
`- [x]` for "render opt-in self-registration" — but no `Register` page exists under
`templates/platform-application-starter/Pages/Identity/Account/`. A checked task for a
rendered file that was never rendered is the failure mode this repository already
diagnoses elsewhere: a check that reports success while asserting nothing.

## What Changes

- Add `ui/packages/react-auth/`, a React package exporting `LoginForm`, `RegisterForm`,
  `ForgotPasswordForm`, and a logout control, composed from the existing `react-ui`
  primitives rather than new markup.
- Add a `ThemeProvider`/`useTheme` pair that sets `data-theme` on the document element,
  defaults to `prefers-color-scheme`, and persists an explicit user choice.
- Add a versioned JSON Schema for `branding.json` plus a typed loader, so logo,
  navigation, and footer links are project data rather than component code.
- Add a Razor `Register` page to the application template so the `EnableSiteUsers`
  self-registration claim is true, and correct the archived task record.

## Package Boundary and Split Assessment

The React package is independently adoptable: it depends on `@platform/react-ui` and
`@platform/design-tokens`, both already published, and on nothing else. The Razor page is
a separate surface with a separate owner (the template pack) and a separate oracle
(generated-app route tests). Splitting them lets a TypeScript-only product adopt the
components without taking a .NET dependency, and lets a Razor-only product take the page
without adopting React.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `@platform/react-auth` | Ship shared auth page components, theme switching, and branding consumption | dotnet-platform-libs, TypeScript/React | `ui/packages/react-auth` package surface | `@platform/react-ui`, `@platform/design-tokens` | package contract tests, `verify-ui.mjs` |
| application template `Register` page | Render the self-registration page the `EnableSiteUsers` symbol already promises | dotnet-platform-libs, C#/.NET 10 | template option `EnableSiteUsers` | existing template, ASP.NET Core Identity | generated-app route test |

The `ui/feed/` tarballs are regenerated in a second commit in this change, after the source
lands, so `@platform/react-auth` becomes externally consumable and the digests recorded in
`feed.json` match the committed bytes. `eng/package-manifest.json` gains no row:
`scripts/generate-package-manifest.sh` reads only `src/**/*.csproj`.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `ui/packages/react-ui` | `src/components.tsx` | `Button`, `Field`, `Card`, `Badge`, `State`, `Dialog`, `Pagination`; `Field` already wires `aria-describedby` and `aria-invalid` | no form or auth composition exists | dotnet-platform-libs UI layer | **compose**; do not fork or re-declare the primitives |
| `ui/tokens/tokens.json` | DTCG tree, `theme.dark.color` | the only legal source of a color value, for every client | dark overrides are the sole place a theme may live | dotnet-platform-libs UI layer | **write only when measurement demands a token**: no literal in new CSS or TSX, and no existing value changed; two semantic colors added after measuring the auth pages |
| `platform-site-user-auth-starter` (archived) | `specs/.../spec.md`, `tasks.md` 2.4 | route contract, generic-response rule, self-registration off by default | its self-registration task is checked but unrendered | dotnet-platform-libs template pack | **modify** the Register page only; keep the archived decisions |
| `Platform.UI.Razor` | `wwwroot/platform-tokens.css` | hand-authored .NET token CSS | carries its own copy of five dark hexes (known boundary, `ui/README.md`) | dotnet-platform-libs | **keep local**; not a dependency of this change |

## BFS Impact Map

- **Consumers:** any product adopting `@platform/react-auth`; new ASP.NET Core sites
  generated with `EnableSiteUsers=true` get the `Register` page. Sites already generated
  are unaffected until they re-generate.
- **Generated state:** `templates/platform-application-starter/Pages/Identity/Account/Register.cshtml`
  and `.cshtml.cs` appear only when `EnableSiteUsers=true`, because the existing `excludes`
  block already removes `Pages/Identity/**` when it is false.
- **Public API:** `@platform/react-auth` is a new package. No existing `Platform.*`
  assembly is touched and no existing token name or value changes, so the
  emitted-token baseline stays valid. The emitted set grows by two additive semantic
  colors, which moves the generated `tokens.css` bytes and therefore the pinned
  `GeneratedTokensCssSha256` and the feed digests.
- **Dependency direction:** `react-auth` → `react-ui` → `design-tokens`. The dependency
  points one way; `react-ui` does not know auth exists.
- **Failure/security:** branding data that fails schema validation is a startup error, not
  a silently blank header. The `Register` page returns one generic response for an unknown
  email and for a password mismatch, matching the existing Login page rule.
- **Compatibility:** `EnableIdentity` and `EnableAdmin` are untouched. The Razor page adds
  a route only; it changes no existing route's behavior.

## Capabilities

- **New:** `platform-shared-auth-pages`.
- **Modified:** none. The archived `platform-site-user-auth-starter` capability keeps its
  spec; this change corrects a task record, not a requirement.

## Non-goals

- No non-.NET-stack auth templates, no social/OIDC login, no multi-tenant identity, and no
  passkey or MFA UI. These are on the archived change's deferred list and stay there.
- No migration of existing products to the new package.
- No new runtime dependency, and no change to any existing token name or value. Two new
  semantic colors are added (`semantic.color.onPrimary`, `semantic.color.border`) because
  the measured contrast failed without them; `ui/feed/` is regenerated so the new package
  is consumable.
- No hand-authored theme palette: the dark theme remains token-driven.
