# control-plane-portal delta

## ADDED Requirements

### Requirement: Forge-administrator password rotation in the topbar

The control-plane portal SHALL expose a `Change password` control
in the dashboard topbar that posts the operator's new password to
`POST /v1/admin/identity/change-password` and, on success,
auto-signs-out so the operator signs back in with the rotated
credential. The control SHALL be visible only when the operator has
a valid `forge_admin_session` cookie; the underlying Core call
SHALL refuse to update a registry with no `forge_admin` row.

#### Scenario: Operator rotates the Forge administrator password

- **WHEN** the operator clicks `Change password` and submits a
  matching current/new/confirm trio
- **THEN** the control posts to the named route, the response is
  `200 { changed: true, contract: "forge-admin-login/1.0.0" }`,
  and the dashboard returns the operator to the sign-in page so
  they re-authenticate with the new password.

#### Scenario: Rotation refusal preserves the existing credential

- **WHEN** the operator submits a weak new password, a
  mismatched confirmation, or a wrong current password
- **THEN** the route returns a typed refusal (`400
  admin-password-too-weak` / `400
  admin-password-confirmation-mismatch` / `403
  admin-current-password-incorrect`), the stored password hash
  is unchanged, and every pre-rotation session is still valid.

#### Scenario: Unconfigured registry refuses to initialize

- **WHEN** the operator submits against a registry with no
  `forge_admin` row
- **THEN** the route returns `403
  admin-identity-not-configured` and never inserts a row.
