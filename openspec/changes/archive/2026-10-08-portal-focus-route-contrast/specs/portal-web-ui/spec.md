# portal-web-ui (delta)

## ADDED Requirements

### Requirement: Focus moves to the main content region on dashboard route change

After the dashboard router switches views, keyboard and screen-reader
focus SHALL move to the `#main-content` region. Same-view parameter
reconciliations (reload, back/forward across `?project=` values, fleet row
actions that stay on the view) SHALL NOT move focus. The move SHALL NOT
alter the URL, history entries, scroll restoration, or the deep-link
parameter handling owned by `fleet-manage-deep-link`.

#### Scenario: Sidebar navigation announces the new view

- **WHEN** the operator activates a sidebar link to a different view
- **THEN** the new view is shown and focus is on the `#main-content`
  region, so assistive technology announces the new view landmark

#### Scenario: Same-view project change keeps focus

- **WHEN** the operator follows a fleet row action that stays on the
  current view with a different `?project=`
- **THEN** the scoped card or workbench detail reconciles as before and
  focus is not moved out of the operator's current control

#### Scenario: Deep-link reload and back/forward survive

- **WHEN** the operator reloads `/workbench?project=<id>` or traverses
  back/forward across two managed ids
- **THEN** the URL's project still boots and no hand selection is
  clobbered by the focus behavior

### Requirement: Keyboard focus is never obscured by the topbar

The sticky topbar SHALL NOT cover a keyboard-focused target. The document
SHALL declare a `scroll-padding-top` offset of at least the topbar height
plus breathing room, and the main content region and titled in-view
sections SHALL carry a matching `scroll-margin-top`, meeting the WCAG 2.2
focus-not-obscured minimum.

#### Scenario: Tab reaches content below the sticky bar

- **WHEN** the operator tabs through the dashboard with the sticky topbar
  visible
- **THEN** every focused target scrolls fully into view below the bar and
  no focused control or heading is hidden behind it

### Requirement: Both auth and dashboard pages pair their color-scheme declaration with the dark token set

The login page and the dashboard shell SHALL declare the same
`color-scheme` value that the shared dark token set implements, and every
normal-text pair in that theme SHALL meet at least 4.5:1 contrast.
Secondary (`--faint`) text on badge-chip backgrounds SHALL meet 4.5:1.

#### Scenario: Login and dashboard agree on the dark theme

- **WHEN** the operator opens `login.html` or any dashboard route
- **THEN** both documents declare the dark scheme and render secondary,
  badge, placeholder, and muted text at 4.5:1 or better in that theme

### Requirement: Login inputs show a visible keyboard focus indicator

Login form inputs SHALL show a focus indicator with at least a 2px
perimeter at 3:1 contrast against the input background in addition to any
border-color change. No input SHALL suppress its outline without providing
that indicator, and every `:focus-visible` rule SHALL resolve to a defined
focus token.

#### Scenario: Tab into the password field

- **WHEN** the operator tabs to a login input
- **THEN** a 2px outline at 3:1 or better is visible around the input
  whether or not the border color also changes
