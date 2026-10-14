# control-plane-portal delta

## ADDED Requirements

### Requirement: Responsive layout and reduced-motion rendering

The control-plane SPA portal SHALL render usable at viewports down to
320 CSS px and SHALL honor the `prefers-reduced-motion` user setting.
At `width <= 720px` the portal SHALL collapse the workbench multi-card
grid to a single column and SHALL reflow the 8-column Bulk Workspace
onboarding table (`#ws-table-wrap` / `#ws-rows`) into one stacked card
per row, preserving every one of its eight columns (Onboard, Directory,
Project id, Action, Profile signal, State, Id override, Profile
override) with a readable per-column label, without removing any column
or introducing a horizontal-scroll-only escape. Every interactive
control SHALL remain keyboard-reachable and SHALL keep a `>=44px` touch
target. Under `prefers-reduced-motion: reduce` the portal SHALL disable
transitions and animations on the lifecycle rail, panels, cards, and
the shared button/nav/action controls without hiding or disabling any
control.

#### Scenario: Workbench grid collapses to one column on a phone

- **WHEN** the viewport width is `<= 720px`
- **THEN** `.workbench-grid` lays out as a single column so no card is
  squished below a readable width, and the sidebar remains a reachable
  top row with all nav destinations in the tab order.

#### Scenario: Bulk onboarding table reflows to stacked cards

- **WHEN** the viewport width is `<= 720px` and the operator opens the
  Bulk Workspace onboarding table
- **THEN** each candidate renders as a stacked card where every one of
  the eight columns is shown with a visible label (via `td::before`
  `:nth-child` rules, no `data-label` required), the Onboard checkbox
  and the Id/Profile override text inputs stay focusable and `>=44px`,
  and the table region keeps its `role="region"` and
  `aria-label="Workspace candidates"` with the column headers still
  exposed to assistive technology.

#### Scenario: Reduced motion suppresses transitions without losing function

- **WHEN** the OS requests `prefers-reduced-motion: reduce`
- **THEN** transitions and animations on the lifecycle rail, panels,
  cards, and the button/nav/action controls are disabled, and every
  control, card, and the rail remain fully present and operable.

#### Scenario: Touch targets stay reachable on narrow screens

- **WHEN** the viewport width is `<= 720px`
- **THEN** all controls retain keyboard reach and a `>=44px` minimum
  touch target, with no page-level horizontal scroll introduced by the
  new breakpoint.
