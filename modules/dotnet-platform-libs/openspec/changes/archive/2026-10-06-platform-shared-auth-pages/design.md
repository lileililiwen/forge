# Design: Shared auth page components, theme switching, and branding metadata

## 1. Implementation boundary

Repository `/home/paul/code/dotnet-platform-libs`. This change adds
`ui/packages/react-auth/` and `ui/schemas/branding.schema.json` inside the `ui/` design-token
layer, and one page pair under `templates/platform-application-starter/Pages/Identity/Account/`.
It adds no `Platform.*` assembly and touches no `src/` project.

Measuring the contrast claim (requirement 4) rather than asserting it found two pairs that
genuinely fail WCAG 2.2 AA, so the change **adds two semantic tokens** to
`ui/tokens/tokens.json` instead of shipping a false claim. No existing token value changes, so
`ui/tokens/emitted-baseline.json` and the 38 baselined 0.1.0 names are untouched; the emitted
set grows from 47 to 49.

| Pair | Before | After |
| --- | --- | --- |
| Button label on the primary fill, dark | white on `color.blue.300` = **1.80:1** | `--color-on-primary` (`color.slate.900`) on `--color-primary` = **9.90:1** |
| Field and divider edge on the surface, light | `color.slate.300` on white = **1.48:1** | `--color-border` (`color.slate.500`) on `--color-surface` = **4.76:1** |

One raw primitive cannot serve the border role in both modes — `slate.500` measures 2.18:1 on
the dark surface and `slate.300` measures 1.48:1 on the light one — so the border is a semantic
token re-pointed per mode, which is the mechanism `theme.dark.color` already exists for.

`ui/feed/` is repacked in its own commit, after the source, so `@platform/react-auth` becomes
externally consumable and the recorded digests match the committed bytes.
`eng/package-manifest.json` is deliberately untouched: `scripts/generate-package-manifest.sh`
reads only `src/**/*.csproj`, so an npm package has no row to add.

## 2. Language and runtime

TypeScript sources under `ui/packages/react-auth/src/`, styled by one hand-authored CSS
file in the same directory, matching how `ui/packages/react-ui/styles.css` is authored.
`react` is a peer dependency, exactly as in `@platform/react-ui`, so the package adds no
runtime dependency of its own. The Razor side is ASP.NET Core 10, matching the template
pack's pinned SDK and the existing `Login.cshtml.cs` / `Logout.cshtml.cs` it sits beside.

Tests are plain `node` scripts in the `react-ui` style: `node tests/<name>.test.mjs`, run
directly. That is the whole existing test harness for the `ui/` layer, and this change does
not add a test framework.

## 3. Ownership and shared code

`ui/packages/react-auth` owns auth page composition, the theme control, and branding
consumption. It owns no visual value: every color, space, radius, and font comes from
`@platform/design-tokens` through `var(--token)`.

`ui/packages/react-ui` is consumed, never forked. `Field` already emits
`aria-invalid` and `aria-describedby` and already sets `aria-live` on its message; a
second implementation of that wiring in `react-auth` would be a second accessibility
contract with a different failure mode, which is the drift this change exists to remove.
`react-auth` therefore passes `error`/`helper` through and adds neither attribute.

Dependency direction is `react-auth` → `react-ui` → `design-tokens`, one way.
`react-ui` gains no knowledge of auth, so the primitive package stays adoptable on its own.

## 4. Behavioral model

`src/auth-forms.tsx` exports `LoginForm`, `RegisterForm`, `ForgotPasswordForm`, and
`LogoutControl`. Each form holds a `state` field over `idle | invalid | submitting |
success | error` and an `errors` record keyed by field name. Submit moves `idle`/`invalid`
→ `invalid` when the local check fails without calling the handler, and → `submitting`
while the awaited handler runs. The handler's return value decides `success` or `error`.
Each form renders through `Card`, its inputs through `Field`, its submit through `Button`
(so `loading` supplies `aria-busy` and blocks a double submit), and its failure through
`State`, whose `error` kind already carries `role="alert"`.

`src/theme.tsx` exports `ThemeProvider` and `useTheme`. It resolves a theme in this order:
an explicit stored choice, then `prefers-color-scheme` when `matchMedia` reports no stored
override, then `light` as the last resort for environments without `matchMedia`. The
resolved value is written to `document.documentElement.dataset.theme`, so the emitted token
CSS under `[data-theme="dark"]` selects the dark overrides with no extra stylesheet. The
control is a `Button` that toggles the value, carries `aria-pressed` to report the current
selection, and renders the current theme name as text so the state is never conveyed by
color alone.

`src/branding.ts` exports the `Branding` type and a `parseBranding` function. The loader
does not read the filesystem: it validates a `branding.json` document already loaded by
the host, which is what "consumed at render time" means for a React component and keeps the
package free of a Node-only dependency. Validation is structural and explicit, checking the
same required fields and types as the JSON Schema, and it throws naming the offending field
path rather than returning a partially valid object. `navigation[]` is ordered by the
numeric `order` field; `footerLinks[]` is grouped by `group`, which is the 友情链接
requirement expressed as data.

## 5. Contract and compatibility

`@platform/react-auth@0.1.0` exports from its root: `LoginForm`, `RegisterForm`,
`ForgotPasswordForm`, `LogoutControl`, `ThemeProvider`, `useTheme`, `ThemeToggle`, and the
branding types. `files` is `["src", "styles.css"]` and `exports` points at the built
output, matching `@platform/react-ui`.

The new `ui/schemas/branding.schema.json` is a draft 2020-12 schema. It is additive: no
existing schema is changed. The two new semantic color tokens are additive too — the 38
baselined names keep their exact values, so `ui-token-renamed` cannot fire — but they do
change the emitted `tokens.css` bytes, so `GeneratedTokensCssSha256` in
`tests/Platform.Architecture.Tests/UiDistributionTests.cs` is updated in this change, which is
the review that constant exists to force. The feed digests move with it and are repacked.

The Razor side adds `Register` under the existing `Pages/Identity/**` exclude, so
`EnableSiteUsers=false` output is byte-identical to today. `SiteUsers:SelfRegistrationEnabled`
keeps the archived default of disabled; the page reads the flag, renders no form without it,
and re-checks it in the POST handler so the route cannot be posted to directly. Every refused
submission — a password mismatch, a password the policy rejects, an email the store already
knows — returns one `GenericRefusal` constant, so the response reveals nothing about which
condition occurred. A genuinely new email creates the account through the same
application-owned `UserManager<ApplicationUser>` every other identity operation uses; no
platform-owned user entity or migration is introduced. The form posts with
`@Html.AntiForgeryToken()`, so CSRF protection is the existing Razor Pages mechanism rather
than a new one.

## 6. Failure and boundary policy

Branding document fails validation → throw, naming the field path; a blank header is not a
recoverable outcome. Stored theme choice is unreadable → treat as absent and follow the
system preference. No `matchMedia` → `light`. A form handler that throws → the form's
`error` state, with no internal detail surfaced. Registration with an unknown email or a
mismatched password → identical generic response, so the page reveals nothing about which
accounts exist.

## 7. Verification oracle

`ui/packages/react-auth/tests/` holds node contract tests in the `react-ui` style, covering
form state transitions, theme persistence and the system default, branding override of
logo, navigation, and footer links, and the accessibility invariants (composed primitives,
label wiring, reduced-motion block, focus-visible, 24px targets).

Contrast is **measured, not asserted on names**. The test resolves the token values out of
`ui/tokens/tokens.json` through the same `readTokens`/`resolveAliases`/`readThemeOverrides`
path in `ui/scripts/lib/token-model.mjs` that the generator and `verify-ui.mjs` use, applies
the `theme.dark` override map for the dark palette, and applies the WCAG relative-luminance
formula. Every foreground/background pair the auth pages actually draw is checked against
4.5:1 for text and 3:1 for UI components and the focus indicator. No hex value is written in
the test, so it cannot pass by agreeing with itself about a colour it invented. A final
guard requires every colour custom property named in `styles.css` to appear in a measured
pair, so a new colour cannot reach the page unmeasured.

The test is proved load-bearing: pointing `theme.dark.color.onPrimary` back at `color.white`
fails it with `dark the submit control label: --color-on-primary (#ffffff) on
--color-primary (#93c5fd) measures 1.80:1, below the 4.5:1 minimum`.

`verify-ui.mjs` remains the token oracle and is re-run before and after `generate-tokens.mjs`
to prove the new package neither dropped a token nor introduced an orphan literal. For the
Razor side the oracle is the generated app's own route tests in
`templates/platform-application-starter/tests/StarterApp.Tests/SiteUserAuthTests.cs` (refusal
while the toggle is off, refusal that creates nothing, creation when enabled, one generic
message for a mismatch and for a known address, and a 400 without an antiforgery token) plus
the template render assertions in `tests/Platform.Template.Tests/SiteUserAuthTemplateTests.cs`.

## 8. Decision ledger

Resolved: the auth package composes the existing primitives instead of forking them; the
theme is a `data-theme` attribute over the existing token tree, so the dark theme keeps a
single source; branding is validated data consumed at render time, not component
configuration; the Razor `Register` page is a separate surface with a separate oracle;
self-registration stays off by default.

Resolved, from requirement 4: contrast is measured from resolved token values rather than
asserted from token names, because the name-based assertion passed against a page that
measured 1.80:1 on its primary button. That measurement forced two additive semantic tokens
rather than a softened claim. Pointer targets use `24px`, not `1.5rem`, so the minimum holds
at a reduced root font size.

Resolved: the archived `platform-site-user-auth-starter` task 2.4 checkbox is corrected in
place, and the correction is annotated rather than silent. The repository has no convention
against editing an archived package — `git log -- openspec/changes/archive/` shows archived
folders are amended when a later finding lands — and the spec for requirement 5 states the
outcome directly ("it no longer claims a completed render for a page that did not exist").
Leaving a known-false checkbox in place to protect an archive boundary would be the
`contract-parity: OK` failure class this repository already names. Nothing else in that
archived package is touched.

Deferred, unchanged from the archived change's ledger: non-.NET stack auth templates,
social/OIDC login for site users, multi-tenant identity, passkey/MFA UI, and migration of
existing products. Consumer adoption of `@platform/react-auth` outside this repository stays
out of scope, as `ui/README.md` already records for the whole layer. Blockers: none.
