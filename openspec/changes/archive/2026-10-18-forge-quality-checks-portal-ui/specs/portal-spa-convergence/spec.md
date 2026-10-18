# portal-spa-convergence Specification (delta: quality-checks-portal-ui)

## MODIFIED Requirements

### Requirement: Per-section SPA deep-link pointer

Forge SHALL render, for each of the twelve control-plane portal sections, the
matching SPA deep-link and coverage verdict in both the human and JSON
projections of `forge portal dashboard` and `forge portal view <section>`:
`projects` → `/projects` (covered), `features` → `/workbench` (covered),
`components` → `/projects` (covered), `policies` → `/workbench` (partial),
`specs` → `/projects` (covered), `agents` → `/projects` (covered),
`deployments` → `/delivery` (covered), `repositories` → `/management`
(partial), `documentation` → CLI-only, `analytics` → `/projects` (partial),
`servers` → CLI-only, `settings` → `/workbench` (covered; displayed as
**Quality**). CLI-only sections SHALL render `spa: (none — CLI only)` and
never name a deep-link no view serves. The mapping SHALL live in one pure
`PortalSection` table with no I/O, and the two JSON fields SHALL be additive
(`#[serde(default)]`, contract version unchanged). The legacy `settings`
identifier SHALL remain accepted and serialized for compatibility.

#### Scenario: Every section view names its SPA pointer

- **WHEN** an operator runs `forge portal view <section>` for each of the
  twelve sections
- **THEN** the human output carries `spa: <route>` and
  `web: <covered|partial|cli-only>` lines matching the mapping above, and the
  JSON view carries equal `spa_route` and `web_coverage` fields

#### Scenario: Dashboard header names the single interactive surface

- **WHEN** an operator runs `forge portal dashboard <target>`
- **THEN** the human output carries one header line naming the SPA dashboard
  served by `forge web` (routes /projects /workbench /management /portfolio
  /delivery) as the interactive surface and the legacy server-side HTML as a
  read-only pointer

#### Scenario: Old JSON still parses

- **WHEN** a pre-change portal JSON payload without the two fields is
  deserialized
- **THEN** parsing succeeds with empty defaults and the contract version
  still reads `0.1.0`
