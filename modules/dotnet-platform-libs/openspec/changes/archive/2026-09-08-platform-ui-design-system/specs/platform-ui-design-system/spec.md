# Shared UI design system

## ADDED Requirements

### Requirement: UI tokens SHALL be the cross-client visual contract

The platform SHALL generate equivalent semantic tokens for React, CSS, and Razor consumers.

#### Scenario: Application changes brand accent

- **WHEN** the application selects a supported accent/theme
- **THEN** supported clients use the same semantic color, contrast, spacing, and typography contract

### Requirement: Shared components SHALL expose complete interaction states

UI primitives SHALL define accessible keyboard, focus, disabled, loading, error, empty, and
forbidden behavior where applicable.

#### Scenario: Form submission is pending

- **WHEN** a form action is submitted
- **THEN** the shared component exposes a pending state, prevents accidental duplicate submission,
  and preserves accessible status information

### Requirement: Admin navigation SHALL respect backend permissions

The UI shell SHALL consume permission/navigation metadata and hide or disable unavailable
features without treating client checks as authorization.

#### Scenario: Operator lacks a permission

- **WHEN** permission metadata excludes an admin section
- **THEN** the navigation does not present that section
- **AND** the backend still enforces the permission independently
