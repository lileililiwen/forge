# forge-contract-defect-remediation Specification (delta)

## ADDED Requirements

### Requirement: Contract defect remediation

Forge SHALL return the established, canonical wire envelope for the portal admin
reads, and its contract-test suite SHALL assert against that same envelope so the
six delivered quick-win changes are production-verified. Specifically:

- The identity password-rotation handler SHALL revoke only the probe session it
  mints while verifying the current password, and SHALL NOT revoke any other
  session when the new password is rejected; `change_password` remains
  responsible for revoking all sessions on success.
- The portfolio interest read handlers (`list`, `show`, `trend`) SHALL refuse an
  unmanaged project id with `404` and code `portfolio-unmanaged-project`, carry
  `effect: "none"`, and SHALL NOT echo the supplied id.
- An unmanaged project id routed through admin dispatch SHALL return `404` with
  code `admin-project-unmanaged` (the canonical envelope used across the admin
  surface).
- A `POST`-only admin route SHALL answer an anonymous `GET` with `405
  method-not-allowed` and an anonymous `POST` with `401 api-unauthorized`.
- The reduced-motion CSS block SHALL name the lifecycle rail, project panels and
  workbench cards and disable their transitions, and SHALL be a non-empty rule.
- The portfolio interest card in `frontend/index.html` SHALL carry the container
  id `portfolio-interest`.

#### Scenario: Weak new password preserves the operator's session

- **WHEN** an authenticated operator submits a correct current password and a
  new password that fails the strength check
- **THEN** Forge returns `400` with code `admin-password-too-weak`, the stored
  password hash is unchanged, the active session count is unchanged, and the
  original credential still authenticates

#### Scenario: Unmanaged project id on interest reads

- **WHEN** an authenticated operator reads `list` / `show` / `trend` for a
  project id that is not managed by the registry
- **THEN** Forge returns `404` with code `portfolio-unmanaged-project`, the
  response carries `effect: "none"`, the supplied id is not echoed, and no
  project is created

#### Scenario: Unmanaged project id via admin dispatch

- **WHEN** an authenticated operator targets an unmanaged project id through an
  admin route such as `/v1/admin/projects/{id}/gate/status`
- **THEN** Forge returns `404` with code `admin-project-unmanaged` (a typed
  envelope, not a route-not-found fallthrough)

#### Scenario: Anonymous request to a POST-only admin route

- **WHEN** an anonymous caller sends `GET` to a `POST`-only admin route
- **THEN** Forge returns `405 method-not-allowed`
- **AND** an anonymous `POST` with `application/json` to that same route returns
  `401 api-unauthorized`

#### Scenario: Reduced-motion coverage is non-empty

- **WHEN** `frontend/styles.css` is shipped
- **THEN** the `prefers-reduced-motion` block names `.lifecycle-rail`,
  `.project-panel` and `.wb-card`, disables their `transition`, and is not an
  empty rule
