# remove-api-ui Specification

## Purpose
Remove the in-process HTML portal at `/ui/*` on the API listener so the standalone web UI on `frontend/` (served by `forge web serve`) becomes the only operator browser path, while the JSON API the web UI calls stays unchanged and every CLI subcommand (including `forge portal`, `forge identity init`/`validate`, and the OIDC `forge identity` subcommands) keeps working.
## Requirements
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

