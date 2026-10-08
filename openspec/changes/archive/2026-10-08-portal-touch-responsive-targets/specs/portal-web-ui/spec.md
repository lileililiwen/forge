# portal-web-ui (delta)

## ADDED Requirements

### Requirement: Operator touch targets are at least 44 CSS pixels tall

Every tappable operator control SHALL present a touch target at least
44 CSS pixels tall: `.button`, `.button-quiet`, `.button-primary`,
`.nav-link`, `.filter-box`, `.fleet-filter input`, `.search-box`,
`.login-form input`, workbench text inputs, `#ws-rows` text inputs, and
`.wb-maintain-decide`. Heights SHALL use `min-height` so taller content
still fits, and existing padding/font-size (visual density) SHALL be
unchanged. Controls whose visual must stay small (checkboxes) SHALL keep
their visual size and meet the target through their activating hit area
(wrapping label rows at 44px).

#### Scenario: Tape-measure audit of operator controls

- **WHEN** the shipped stylesheet is inspected for the listed selectors
- **THEN** each computes `min-height >= 44px` (or a 44px activating
  label/row area for checkboxes) with no reduced padding or font-size

#### Scenario: Dense surfaces keep their density

- **WHEN** the operator views the fleet filter row, workspace candidate
  table, or maintain decisions
- **THEN** row padding, font sizes, and column structure match the
  pre-change design; only hit areas grow

### Requirement: Taps respond without delay and confirm on press

Interactive elements SHALL carry `touch-action: manipulation` so taps
never wait on the legacy tap delay. Tappable elements SHALL show press
feedback within 80–150ms via opacity/elevation change that causes no
layout shift, and every clickable SHALL show `cursor:pointer`.

#### Scenario: Tap a control on a touch device

- **WHEN** the operator taps any button, nav link, filter, input, or
  action head
- **THEN** the press is acknowledged within 150ms with no page-level
  double-tap wait and no sibling movement

#### Scenario: Hover a clickable with a pointer

- **WHEN** the operator hovers any button, nav link, maintain decision,
  or checkbox control
- **THEN** a pointer cursor is shown

### Requirement: Fixed and sticky chrome respects safe areas and the dynamic viewport

Sticky/fixed chrome (`.topbar`, `.sidebar`, `.skip-link`) and the page
body SHALL offset with `env(safe-area-inset-*)` (with zero fallbacks) so
tappables never collide with the notch or gesture bar, and both
documents SHALL declare `viewport-fit=cover`. Viewport-filling minima
SHALL track the dynamic viewport (`100dvh`, each keeping its `100vh`
fallback) so sections stay correct as the mobile URL bar shows/hides.

#### Scenario: Open the dashboard on a notched phone

- **WHEN** the operator loads any dashboard route with a notch/gesture
  bar present
- **THEN** the topbar, sidebar, skip link, and page edges clear the
  insets and no tappable sits under the cutout or home indicator

#### Scenario: Scroll the mobile URL bar

- **WHEN** the mobile browser shows or hides its URL bar
- **THEN** the auth layout and app shell keep filling the visible
  viewport with no jump or gap

### Requirement: Programmatic scroll honors reduced motion

The two programmatic smooth scrolls (Maintain decision prefill,
workbench detail open) SHALL scroll instantly when the operator prefers
reduced motion and smoothly otherwise. No unconditional
`{ behavior: "smooth" }` scroll SHALL remain.

#### Scenario: Reduced-motion user opens a workbench project

- **WHEN** reduced motion is preferred and the workbench detail loads
- **THEN** the title scrolls into view instantly with no animation

#### Scenario: Motion-tolerant user jumps to a Maintain decision

- **WHEN** reduced motion is not preferred and the operator opens a
  Maintain decision
- **THEN** the card scrolls into view smoothly, centered as before
