# portal-spa-convergence Specification

## Purpose
The SPA dashboard served by `forge web` is the single interactive
browser surface; `forge portal dashboard|view` renders the twelve
§36 sections as legacy server-side HTML that names the matching SPA
deep-link and coverage verdict per section, with CLI-only sections
pointing at the terminal instead of a dead link.
## Requirements
### Requirement: Per-section SPA deep-link pointer

Forge SHALL render, for each of the twelve control-plane portal
sections, the matching SPA deep-link and coverage verdict in both the
human and JSON projections of `forge portal dashboard` and
`forge portal view <section>`: `projects` → `/projects` (covered),
`features` → `/workbench` (covered), `components` → `/projects`
(covered), `policies` → `/workbench` (partial), `specs` → `/projects`
(covered), `agents` → `/projects` (covered), `deployments` →
`/delivery` (covered), `repositories` → `/management` (partial),
`documentation` → CLI-only, `analytics` → `/projects` (partial),
`servers` → CLI-only, `settings` → `/projects` (partial). CLI-only
sections SHALL render `spa: (none — CLI only)` and never name a
deep-link no view serves. The mapping SHALL live in one pure
`PortalSection` table with no I/O, and the two JSON fields SHALL be
additive (`#[serde(default)]`, contract version unchanged).

#### Scenario: Every section view names its SPA pointer

- **WHEN** an operator runs `forge portal view <section>` for each of
  the twelve sections
- **THEN** the human output carries `spa: <route>` and
  `web: <covered|partial|cli-only>` lines matching the mapping above,
  and the JSON view carries equal `spa_route` and `web_coverage` fields

#### Scenario: Dashboard header names the single interactive surface

- **WHEN** an operator runs `forge portal dashboard <target>`
- **THEN** the human output carries one header line naming the SPA
  dashboard served by `forge web` (routes /projects /workbench
  /management /portfolio /delivery) as the interactive surface and the
  legacy server-side HTML as a read-only pointer

#### Scenario: Old JSON still parses

- **WHEN** a pre-change portal JSON payload without the two fields is
  deserialized
- **THEN** parsing succeeds with empty defaults and the contract
  version still reads `0.1.0`

### Requirement: Documented coverage vs honestly CLI-only

Forge SHALL document, in `docs/portal-spa.md`, which of the twelve
sections the SPA covers where and which stay honestly CLI-only with
their existing catalog reason keys (transports, git, native toolchain,
secrets, live provider probes, capability-gated observe). No
`forge portal` command SHALL be removed or renamed, and no new SPA
view, API route, write, or dependency SHALL be added by this change.

#### Scenario: Coverage table agrees with the rendered pointers

- **WHEN** an operator reads `docs/portal-spa.md`
- **THEN** each of the twelve rows names the same `spa_route` and
  `web_coverage` the CLI renders for that section, and each CLI-only
  row names the catalog reason key that keeps it in the terminal

#### Scenario: Convergence adds no second surface

- **WHEN** the change diff is reviewed
- **THEN** `src/web.rs`, `frontend/app.js`, the API router, the command
  catalog rows and every Core module are byte-identical except for
  reason-text the catalog integrity suite explicitly named; the only
  SPA-side change is one HTML comment noting the SPA is the single
  interactive surface

