# Proposal: Remove the in-process /ui HTML portal

## Why

`src/api/ui/` ships an HTML portal at `/ui/*` on the
`forge api serve` listener (port 8765). The page is a
plain HTML/CSS form; it is not the standalone web UI
(`frontend/`, served by `forge web serve` on port 4173).
It also routes the per-project OIDC sign-in flow at
`/ui/sign-in`, `/ui/auth/callback`, and `/ui/sign-out`,
which only this portal uses.

The portal is the page that returns 404-shaped JSON
(`{"contract":"0.1.0","error":{"code":"route-not-found",...}}`)
when an unauthenticated browser hits `/ui` and gets
redirected to `/ui/sign-in?return=%2Fui`. The operator's
control plane is the standalone web UI; the in-process
HTML portal duplicates the surface, ships its own auth
mechanism, and is the route the user keeps tripping over
when they expect the web UI to be there.

Removing `src/api/ui/` leaves the JSON API on port 8765
intact (the web UI still calls it), removes the HTML
portal and its OIDC sign-in flow, and keeps the rest of
Forge (the global admin `forge identity init`/`validate`,
the `forge portal` CLI command, the standard library and
the standalone web UI) unchanged.

## What Changes

- `src/api/ui/` is deleted (5 files, ~3,335 lines, plus
  `static/`).
- The `/ui`, `/ui/projects/{id}`,
  `/ui/projects/{id}/publish`, `/ui/sign-in`,
  `/ui/auth/callback`, and `/ui/sign-out` HTTP routes
  return 404 on the API listener.
- Tests that exercised the removed surface are deleted:
  `tests/portal_ui_contract.rs` (1,431 lines),
  `tests/portal_browser_a11y.rs`,
  `tests/forge_portal_frontend_contract.rs`,
  `tests/portal_contract.rs` (cases that hit `/ui/*`),
  `tests/portal_cross_surface.rs` (cases that hit `/ui/*`).
- Specs that describe the removed surface are deleted:
  `forge-web-human-dashboard`, `forge-web-workspace-onboarding`,
  `forge-admin-login` (the OIDC sign-in part; the global
  admin password spec `central-admin-identity` stays).
- The standalone web UI (`frontend/`, served by
  `forge web serve` on port 4173) is unchanged.
- The JSON API on port 8765 is unchanged for every
  non-`/ui/*` route.
- `forge portal` CLI (the `dashboard` and `view`
  subcommands) is unchanged. The data the HTML portal
  showed is the same data the JSON API returns; the web
  UI on port 4173 renders it.
- `forge identity` (global admin password) is unchanged.
  Only the per-project OIDC sign-in path
  (`/ui/sign-in` + `/ui/auth/callback` + `/ui/sign-out`)
  is removed, because it was wired only by the deleted
  HTML portal.

## Package Boundary and Split Assessment

This is one bounded, verifiable removal. The
`forge-web-project-management` and the other
`forge-web-project-*` specs stay: they describe the JSON
API surfaces the standalone web UI calls, and the
in-process HTML portal was only one consumer of those
surfaces.

| Package | Outcome | Boundary | Oracle |
|---|---|---|---|
| `remove-api-ui` (this) | The in-process HTML portal and its OIDC sign-in flow are gone; the JSON API, the standalone web UI, the `forge portal` CLI, and the global admin `forge identity` are unchanged | `src/api/ui/`, the named test files, the three named specs | `cargo build`, `cargo test --workspace`, the web UI running unchanged on port 4173, the JSON API on port 8765 returning the same data as before, `GET /ui/*` returning 404 |

## Sibling and Shared Architecture Reconnaissance

| Surface | Reusable code | Compatibility gap | Decision |
|---|---|---|---|
| `src/api/ui/` (HTML portal + OIDC sign-in routes) | None — every route is consumed only by browser users hitting `/ui/*` | None | **remove** |
| `src/identity/` (OIDC) | `AuthChallenge`, `mint_session`, `save_session`, etc. are also consumed by `tests/identity_contract.rs` (the OIDC round-trip cases) | The OIDC round-trip cases in `tests/identity_contract.rs` are dropped; the `forge identity init` and `forge identity validate` cases stay | **keep** the OIDC machinery; **drop** the OIDC round-trip test cases |
| `src/portal/` (`forge portal` CLI) | None of its calls are routed through `src/api/ui/` | None | **keep** unchanged |
| `frontend/` (standalone web UI) | Uses the JSON API on port 8765; does not call `/ui/*` | None | **keep** unchanged |
| JSON API in `src/api/mod.rs` | Web UI calls `/v1/...` | None | **keep** unchanged |

## User Experience and Interface Impact

`UI/UX: /ui/* users lose the HTML portal, everyone else is unchanged`.

- Operators using the standalone web UI (`forge web serve`
  on port 4173) see no change.
- Operators hitting `/ui` in a browser now get a 404
  (the route registration is gone). They were a small
  minority — the web UI supersedes this.
- Per-project OIDC sign-in cookies (`forge_session`,
  `forge_oidc_state`) are no longer issued by the API
  listener; existing cookies become inert. The CLI
  `forge identity challenge / callback / list / inspect /
  terminate` subcommands are unchanged.

## BFS Impact Map

- **Capabilities removed:** the HTML portal at `/ui/*` and
  its OIDC sign-in flow.
- **Capabilities unchanged:** the JSON API, the
  standalone web UI, the `forge portal` CLI, the
  `forge identity` (global admin) CLI, the rest of
  `src/identity/`.
- **Users / flows:** web UI users unchanged; `/ui/*`
  browser users reroute to the web UI; CLI users
  unchanged.
- **Contracts / data / persistence:** the JSON API contract
  is unchanged; the registry/journal schema is unchanged;
  the `forge.yaml` schema is unchanged.
- **Integrations / configuration:** none.
- **Callers:** the OIDC code in `src/identity/` keeps
  its non-`/ui/*` callers; only the deleted
  `src/api/ui/auth.rs` is removed.
- **Failure / boundary behavior:** the API listener
  returns 404 on `/ui/*`; everything else is byte-identical.
- **Tests:** the named test files are deleted.
- **Privacy / security:** the OIDC code's session and
  challenge storage under `<project>/.forge/identity/`
  is still reachable via the CLI subcommands; the
  deleted routes simply stop issuing cookies.

## Capabilities

This change **removes** the following capabilities. No new
capability is added.

- `forge-web-human-dashboard` (the in-process HTML portal at `/ui/*`)
- `forge-web-workspace-onboarding` (the in-process onboarding flow at `/ui/...`)
- The per-project OIDC sign-in part of `forge-admin-login`

## Non-goals

- No change to the JSON API surface
  (`src/api/mod.rs`), the standalone web UI
  (`frontend/`), or the `forge web serve` command.
- No change to the `forge portal` CLI command.
- No change to the `forge identity init` (global admin
  password) or `forge identity validate` (manifest
  schema check) subcommands.
- No change to the OIDC machinery in `src/identity/`.
  The CLI subcommands that exercise it
  (`forge identity challenge / callback / list / inspect /
  terminate`) keep working; only the HTTP routes
  `/ui/sign-in`, `/ui/auth/callback`, `/ui/sign-out` are
  removed.
- No removal of the `identity:` field from the
  `forge.yaml` schema.
- No bumping of the registry or journal schema versions.
