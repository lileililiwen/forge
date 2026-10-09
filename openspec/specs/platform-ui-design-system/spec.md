# platform-ui-design-system Specification

## Purpose

The C# applications split between React, Razor Pages, and other clients, repeatedly creating buttons, forms, tables, shells, loading/error/empty states, themes, and admin layouts. The FullStackHero admin and dashboard already contain a useful design-token and component reference, but the two clients duplicate much of it.
## Requirements
### Requirement: UI tokens SHALL be the cross-client visual contract

The platform SHALL generate equivalent semantic tokens for React, CSS, and Razor consumers.

#### Scenario: Application changes brand accent

- **WHEN** the application selects a supported accent/theme
- **THEN** supported clients use the same semantic color, contrast, spacing, and typography contract

### Requirement: Shared components SHALL expose complete interaction states

UI primitives SHALL define accessible keyboard, focus, disabled, loading, error, empty,
and forbidden behavior where applicable. Per the adopted brand guideline, every component
guidance SHALL additionally define, and the shipped guidance document SHALL enumerate, the
full visible state set: default, hover, focus-visible, active, disabled, loading, and
error, with the focus-visible state carrying a contrast-verified focus indicator drawn
from the documented focus-ring token. A component or component guidance that omits one of
these states SHALL NOT be considered complete.

#### Scenario: Form submission is pending

- **WHEN** a form action is submitted
- **THEN** the shared component exposes a pending state, prevents accidental duplicate
  submission, and preserves accessible status information

#### Scenario: Component guidance omits a required state

- **WHEN** component guidance does not define one of default, hover, focus-visible,
  active, disabled, loading, or error for a state where it applies
- **THEN** the guidance is reported incomplete against the kit's accessibility rules
  documented in `docs/ui-tokens.md`

#### Scenario: Focus leaves the pointer path

- **WHEN** a user navigates a shared component by keyboard
- **THEN** the component shows a visible `:focus-visible` indicator using the documented
  focus-ring token
- **AND** the indicator meets the recorded non-text contrast threshold in both themes

### Requirement: Admin navigation SHALL respect backend permissions

The UI shell SHALL consume permission/navigation metadata and hide or disable unavailable
features without treating client checks as authorization.

#### Scenario: Operator lacks a permission

- **WHEN** permission metadata excludes an admin section
- **THEN** the navigation does not present that section
- **AND** the backend still enforces the permission independently
