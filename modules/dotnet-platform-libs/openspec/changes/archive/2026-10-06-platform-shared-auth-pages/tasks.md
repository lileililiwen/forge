# Tasks: platform-shared-auth-pages

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map `ui/packages/react-ui/src/components.tsx` primitives, the token names emitted
  by `generate-tokens.mjs`, and the `styles.css` authoring style.
- [x] 1.2 Confirm the `EnableSiteUsers` symbol description and the
  `Pages/Identity/**` exclude, and prove `Register` does not exist today.
- [x] 1.3 Record the baseline: `verify-ui.mjs` passes and the emitted-token set matches
  `ui/tokens/emitted-baseline.json` before any new file exists.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add `ui/packages/react-auth/` with `package.json`, `src/index.ts`, and the
  composed `LoginForm`, `RegisterForm`, `ForgotPasswordForm`, and `LogoutControl` over
  `Button`/`Field`/`Card`/`State`, declaring `idle`/`invalid`/`submitting`/`success`/
  `error`.
- [x] 2.2 Add `src/theme.tsx` with `ThemeProvider`, `useTheme`, and the keyboard-operable
  toggle, defaulting to `prefers-color-scheme` and persisting an explicit choice.
- [x] 2.3 Add `ui/schemas/branding.schema.json` and the paired `src/branding.ts` typed
  loader, consumed at render time for logo, navigation, and grouped footer links.
- [x] 2.4 Add the package CSS reading only `var(--token)` values, including the reduced-
  motion block, focus-visible rule, and 24px pointer targets.
- [x] 2.5 Render `templates/platform-application-starter/Pages/Identity/Account/Register.cshtml`
  and `.cshtml.cs` honouring `SiteUsers:SelfRegistrationEnabled`, the generic response, and
  CSRF, and correct archived task 2.4 to stop claiming an unrendered page.
- [x] 2.6 Audit the auth pages for WCAG 2.2 AA at 320 CSS px: contrast, unobscured focus,
  skip-to-content link, programmatic labels, and reduced motion.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Prove no new token is dropped and no orphan literal is introduced: run
  `verify-ui.mjs` before and after `generate-tokens.mjs`.
- [x] 3.2 Prove the package adds no runtime dependency beyond the `react` peer dependency
  already used by `@platform/react-ui`.
- [x] 3.3 Prove `EnableSiteUsers=false` template output is unchanged by the Register page.
- [x] 3.4 Repack `ui/feed/` so `@platform/react-auth` is consumable, and confirm
  `eng/package-manifest.json` needs no row: it is generated from `src/**/*.csproj` only,
  so an npm package does not belong in it.

## 4. Verification

- [x] 4.1 Run the `react-auth` tests, `node ui/scripts/verify-ui.mjs`, strict OpenSpec
  validation, and `git diff --check`.
- [x] 4.2 Record the feed regeneration in its own commit.

Remaining: none. `enableSiteUsers=false` output is unchanged because the Register page
rides the pre-existing `Pages/Identity/**` exclude rather than adding a new one.
