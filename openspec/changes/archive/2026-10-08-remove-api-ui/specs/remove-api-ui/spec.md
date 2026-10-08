# remove-api-ui (delta)

## REMOVED Requirements

### Requirement: In-process /ui HTML portal
- **Driver:** `forge-web-human-dashboard`
- **Source spec:** `openspec/specs/forge-web-human-dashboard/spec.md`
- **Surfaces removed:**
  - The `src/api/ui/` directory (`mod.rs`, `auth.rs`,
    `data.rs`, `render.rs`, `routes.rs`, plus `static/`).
  - The `/ui`, `/ui/projects/{id}`,
    `/ui/projects/{id}/publish`, `/ui/sign-in`,
    `/ui/auth/callback`, `/ui/sign-out` HTTP routes.
- **Verification oracle:** the named tests in
  `tests/portal_ui_contract.rs`,
  `tests/portal_browser_a11y.rs`, and
  `tests/forge_portal_frontend_contract.rs` are removed;
  the new behavior is that `GET /ui/*` returns 404 on
  the API listener.

### Requirement: In-process onboarding HTML flow
- **Driver:** `forge-web-workspace-onboarding`
- **Source spec:** `openspec/specs/forge-web-workspace-onboarding/spec.md`
- **Surfaces removed:** the onboarding HTML flow that
  lived in the deleted `src/api/ui/` directory.
- **Verification oracle:** the corresponding tests in
  `tests/forge_web_workspace_onboarding_contract.rs`
  that hit the deleted routes are removed; the routes
  return 404.

### Requirement: OIDC sign-in part of forge-admin-login
- **Driver:** `forge-admin-login`
- **Source spec:** `openspec/specs/forge-admin-login/spec.md`
- **Surfaces removed:** the per-project OIDC sign-in
  HTTP routes (`/ui/sign-in`, `/ui/auth/callback`,
  `/ui/sign-out`) that lived in the deleted
  `src/api/ui/auth.rs`. The OIDC CLI subcommands
  (`forge identity challenge / callback / list /
  inspect / terminate`) and the global admin password
  surface (`forge identity init`) are unchanged.
- **Verification oracle:** the OIDC round-trip cases in
  `tests/identity_contract.rs` that hit the deleted
  routes are removed; the routes return 404. The
  non-`/ui/*` cases (CLI subcommands, manifest schema
  validation) stay.

## ADDED Requirements

### Requirement: Web UI is the operator-facing path
The standalone web UI at `frontend/` (served by
`forge web serve` on the default port 4173) SHALL be the
operator's only browser-facing control plane. The
in-process HTML portal at `/ui/*` SHALL be removed; the
JSON API on `forge api serve` SHALL be unchanged and
SHALL be what the web UI talks to.

#### Scenario: Web UI loads and exercises the JSON API
- **WHEN** an operator runs `forge web serve` and visits
  `http://127.0.0.1:4173/`
- **THEN** the web UI loads, the JS calls
  `http://127.0.0.1:8765/v1/admin/session` and the
  other `/v1/...` endpoints, and the page renders
  the same fleet, project, and command surface as
  before the removal

#### Scenario: The removed routes return 404
- **WHEN** an operator hits `GET /ui`,
  `GET /ui/projects/{id}`,
  `POST /ui/projects/{id}/publish`, `GET /ui/sign-in`,
  `GET /ui/auth/callback`, or `POST /ui/sign-out` on
  the API listener
- **THEN** the response is 404 with the standard
  `route-not-found` error envelope

#### Scenario: The remaining CLI surface is unchanged
- **WHEN** an operator runs `forge portal dashboard`,
  `forge portal view`, `forge identity init`,
  `forge identity validate`, `forge identity challenge`,
  `forge identity callback`, `forge identity list`,
  `forge identity inspect`, or `forge identity terminate`
- **THEN** the behavior is byte-identical to before
  this change
