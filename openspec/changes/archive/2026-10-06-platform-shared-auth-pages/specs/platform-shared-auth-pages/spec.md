## ADDED Requirements

### Requirement: Shared auth page components

The platform SHALL provide a `@platform/react-auth` package exporting `LoginForm`,
`RegisterForm`, `ForgotPasswordForm`, and a logout control. Each form SHALL declare the
states `idle`, `invalid`, `submitting`, `success`, and `error`, and SHALL compose the
existing primitives from `ui/packages/react-ui/src/components.tsx` rather than
hand-rolling form controls.

#### Scenario: Form renders from shared primitives

- **WHEN** a consumer renders `LoginForm`
- **THEN** the emitted markup contains `Button` and `Field` composition from
  `ui/packages/react-ui/src/components.tsx` and no hand-written `<input>` element

#### Scenario: Field validation wiring is reused

- **WHEN** a field is in the `invalid` state
- **THEN** the existing `Field` primitive supplies `aria-invalid` and `aria-describedby`
  and `react-auth` adds neither attribute itself

#### Scenario: Form declares its full state set

- **WHEN** each exported form declares its states
- **THEN** `idle`, `invalid`, `submitting`, `success`, and `error` are all declared and
  reachable

#### Scenario: Submission is pending

- **WHEN** a form is in the `submitting` state
- **THEN** the submit control exposes a pending state, prevents duplicate submission, and
  the transition is announced to assistive technology

#### Scenario: Logout is a control, not a navigation

- **WHEN** the logout control is rendered
- **THEN** it is operable by keyboard and invokes an explicit sign-out handler

### Requirement: Theme switching

The platform SHALL provide a `ThemeProvider` and a `useTheme` hook that set
`data-theme="light"` or `data-theme="dark"` on the document element, SHALL default to
`prefers-color-scheme` when the user has expressed no preference, and SHALL persist an
explicit user choice. Theme styling SHALL read only from `ui/tokens/tokens.json` through
`var(--token)`.

#### Scenario: No stored preference

- **WHEN** a visitor loads a page with no stored theme preference
- **THEN** the resolved theme follows `prefers-color-scheme`

#### Scenario: Explicit choice is persisted

- **WHEN** the visitor selects a theme
- **THEN** `data-theme` is set on the document element and the choice is persisted, so a
  later load resolves the same theme without asking the system again

#### Scenario: Choice can be cleared

- **WHEN** the visitor clears the stored preference
- **THEN** the theme returns to `prefers-color-scheme`

#### Scenario: Control is keyboard operable

- **WHEN** the visitor reaches the theme control with a keyboard only
- **THEN** the control can be focused and activated, and the current theme is exposed as
  text or state to assistive technology

#### Scenario: No hardcoded color

- **WHEN** the theme styling is inspected
- **THEN** every color is a `var(--token)` reference declared in
  `ui/tokens/tokens.json`, and no color literal appears in the package's CSS or TSX

### Requirement: Branding metadata

The platform SHALL define a JSON Schema at `ui/schemas/branding.schema.json` describing
`branding.json` with `name`, `logo` (path or inline data), `theme` (light/dark default),
`navigation[]` (`{id,label,href,order,icon?}`), `footerLinks[]`
(`{id,label,href,group}`), and `locale`. Branding SHALL be consumed at render time by a
typed loader at `ui/packages/react-auth/src/branding.ts`, so a project replaces logo,
navigation, and footer links without editing component code.

#### Scenario: Valid branding document

- **WHEN** a `branding.json` matches the schema
- **THEN** the loader returns typed navigation and footer links and the rendered logo,
  navigation, and footer links come from that document

#### Scenario: Missing required field

- **WHEN** a `branding.json` omits `name`, `logo`, or any other required field
- **THEN** the document fails validation and the failure is reported rather than rendering
  a partially branded page

#### Scenario: Footer link group

- **WHEN** `footerLinks[]` contains entries with a `group` value
- **THEN** the rendered footer groups those links by `group`, covering the 友情链接
  ("friendly links") requirement

#### Scenario: Invalid document is rejected

- **WHEN** a `branding.json` has a wrong field type, an unknown `theme` value, or a
  navigation entry missing `order`
- **THEN** the loader rejects it and does not silently fall back to defaults

### Requirement: Responsive and accessible auth pages

Auth pages SHALL reflow at a 320 CSS pixel viewport and SHALL meet WCAG 2.2 AA: text
contrast of at least 4.5:1, UI component and large-text contrast of at least 3:1, a
visible focus indicator that is not obscured, a skip-to-content link, programmatic
labels, honoured `prefers-reduced-motion`, and pointer targets of at least 24 by 24 CSS
pixels.

#### Scenario: Narrow viewport

- **WHEN** the auth page is rendered at 320 CSS pixels wide
- **THEN** no horizontal scrolling is required and all controls remain reachable

#### Scenario: Text and UI contrast

- **WHEN** the light and dark themes are measured against their token values
- **THEN** text meets 4.5:1 and UI component and large text meet 3:1

#### Scenario: Focus is visible and unobscured

- **WHEN** any interactive element receives keyboard focus
- **THEN** a focus indicator is visible and is not covered by sticky or overlay content

#### Scenario: Skip to content

- **WHEN** the page is loaded and the first control is activated by keyboard
- **THEN** a skip-to-content link moves focus to the main content region

#### Scenario: Every control is labelled

- **WHEN** an audit walks the auth pages
- **THEN** every control has a programmatic label and every input has a programmatically
  associated description when a hint or error is present

#### Scenario: Reduced motion is honoured

- **WHEN** the visitor's system requests `prefers-reduced-motion: reduce`
- **THEN** non-essential transitions and animation are suppressed

#### Scenario: Pointer target size

- **WHEN** a pointer target is measured
- **THEN** it is at least 24 by 24 CSS pixels, or an equivalent spacing exception is
  documented

### Requirement: Razor self-registration page

The application template SHALL render
`templates/platform-application-starter/Pages/Identity/Account/Register.cshtml` and its
code-behind when `EnableSiteUsers` is enabled, so the `EnableSiteUsers` symbol's documented
self-registration is true. The page SHALL respect `SiteUsers:SelfRegistrationEnabled`,
SHALL return a generic response for an unknown email and for a password mismatch, and
SHALL keep CSRF protection.

#### Scenario: Self-registration disabled by default

- **WHEN** a site is generated with `EnableSiteUsers=true` and
  `SiteUsers:SelfRegistrationEnabled` is not set to `true`
- **THEN** registration is refused, matching the archived decision that self-registration
  is off until the application owner enables it

#### Scenario: Self-registration enabled

- **WHEN** the application owner sets `SiteUsers:SelfRegistrationEnabled=true` and a
  visitor submits a valid registration
- **THEN** the account is created through the same application-owned user store as every
  other identity operation, and no platform-owned user entity is introduced

#### Scenario: Unknown email and password mismatch are indistinguishable

- **WHEN** registration is submitted with an unknown email or with a mismatched password
- **THEN** both produce an identical public response that reveals which condition occurred
  no more than the success response does

#### Scenario: CSRF protection is retained

- **WHEN** the registration form is submitted
- **THEN** a request without a valid antiforgery token is rejected

#### Scenario: Archived task record is corrected

- **WHEN** the archived change's task 2.4 is reviewed
- **THEN** it no longer claims a completed render for a page that did not exist
