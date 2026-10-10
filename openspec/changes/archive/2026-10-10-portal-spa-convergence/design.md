# Design: Portal–SPA convergence

## 1. Ownership and placement

The change lives in `src/portal/` (text projection only) plus docs and
test pins. No SPA view/route/dependency change, no API change, no Core
change. `src/web.rs`, `frontend/app.js`, the router table and the
command catalog rows are untouched (reason-text repair only if the
integrity test names a gap).

## 2. Section → SPA mapping

One `PortalSection` method pair in `src/portal/model.rs`, each arm a
pure table with no I/O:

| Portal section | `spa_route()` | `web_coverage()` | Rationale |
|---|---|---|---|
| `projects` | `/projects` | `covered` | Fleet view renders registry + sources + catalog browsers |
| `features` | `/workbench` | `covered` | `feature add/remove/upgrade` are executable catalog rows |
| `components` | `/projects` | `covered` | Creation-catalog browser covers component list/inspect/resolve |
| `policies` | `/workbench` | `partial` | `doctor`/`check` read in workbench; driftwatch policy writes stay CLI |
| `specs` | `/projects` | `covered` | Assurance browser covers spec list/inspect/route; generate/apply executable |
| `agents` | `/projects` | `covered` | Agent/identity readonly browser covers list/status/config/sessions |
| `deployments` | `/delivery` | `covered` | Deploy plan/apply executable rows + deploy/release history |
| `repositories` | `/management` | `partial` | Workspace onboarding in management; git writes (`commit`/`push`/`mirror`) stay CLI-only with reason |
| `documentation` | — (CLI) | `cli-only` | `docs translate` is provider-required; no browser surface by design |
| `analytics` | `/projects` | `partial` | `metrics` read in assurance browser; `inspect` probes live provider planes, CLI-only with reason |
| `servers` | — (CLI) | `cli-only` | `api serve` / `web serve` are loopback transports, CLI-only with reason |
| `settings` | `/projects` | `partial` | Governance list/status/inspect read in assurance browser; `governance use` and portal config stay CLI |

`cli-only` sections render `spa: (none — CLI only)` so the pointer never
names a dead deep-link. `spa_note()` carries the one-line operator
wording (e.g. `policies`: "reads in the workbench; policy writes stay
in the terminal").

## 3. Rendering

- `PortalSectionView` gains `spa_route: String` and `web_coverage:
  String`, both `#[serde(default)]` so pre-change JSON still parses;
  `build_section` populates them from the table. Contract version stays
  `0.1.0` (additive fields only).
- `render_section_human` appends two lines after `source`:
  `spa: <route>` and `web: <coverage> — <note>`.
- `render_dashboard_human` prepends one header line after the title:
  `Interactive surface: the SPA dashboard served by `forge web`
  (routes /projects /workbench /management /portfolio /delivery) —
  this legacy server-side HTML is a read-only pointer.`
- `controls_available` (CLI strings) is unchanged: the pointer is
  additive, never a removal.

## 4. Docs and SPA note

- New `docs/portal-spa.md`: the twelve-row table from §2 plus the
  CLI-only reason keys (`transports`, `git`, `native toolchain`,
  `secrets`, `live provider probes`, `capability-gated observe`).
- `frontend/index.html`: one HTML comment in `<head>` noting the SPA is
  the single interactive surface and the legacy portal HTML is a
  read-only pointer. No functional change.

## 5. Tests

- `src/portal/portal_tests.rs`: struct-literal updates for the two new
  fields; renderer needles for `spa:` / `web:` / the dashboard header.
- `tests/portal_contract.rs`: new tests pinning all twelve
  `(section, spa_route, web_coverage)` triples in JSON plus the human
  header pointer and per-section `spa:` lines; existing tests keep
  passing unchanged otherwise.
- Catalog integrity (`forge_web_command_catalog_contract`,
  `catalog_contract`): rerun to confirm every remaining honest-CLI pin
  still carries a non-empty reason; repair only what the suite names.

## 6. Risks

- A wrong deep-link would send operators to a view that cannot serve
  the section. Mitigated by reusing only the five existing
  navigation-contract routes and pinning each triple in contract tests.
- JSON consumers matching the envelope exactly could see new keys.
  Mitigated: additive-only, version unchanged, `serde(default)` on
  read; the portal JSON is versioned `0.1.0` and additive growth is the
  documented evolution rule.
