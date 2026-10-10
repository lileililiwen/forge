# Proposal: Portal–SPA convergence

## Why

`forge portal dashboard|view` renders twelve server-side HTML sections
(projects/features/components/policies/specs/agents/deployments/
repositories/documentation/analytics/servers/settings, `src/portal/`)
while the browser SPA (`frontend/`) serves projects/workbench/
management/portfolio/delivery plus the reference/catalog/assurance/
history browsers from audit gaps 1–8. Two parallel "portal" surfaces
claim the operator's attention: the legacy HTML reads as a second
interactive surface when it is, by design, a read-only terminal-adjacent
projection. This is audit gap 9 (final).

The SPA is already the complete interactive surface — every portal
section's interactive content either lives there (fleet, workbench
actions, management onboarding, portfolio/delivery controls, catalog,
assurance, history browsers) or is honestly CLI-only for a stated reason
(transports, git, native toolchain, secrets, live provider probes,
capability-gated observe). No new view, route, or write is needed to
converge; the legacy HTML only needs to say so.

## What Changes

- Each of the twelve `PortalSection`s gains a stable SPA deep-link
  (`/projects`, `/workbench`, `/management`, `/portfolio`, `/delivery`)
  plus a coverage verdict (`covered` / `partial` / `cli-only`) with a
  one-line operator note, computed in `src/portal/` with no new
  dependency.
- `forge portal dashboard` / `forge portal view <section>` human output
  names the matching SPA deep-link per section and carries one header
  line naming the SPA (`forge web`) as the single interactive surface;
  the legacy HTML is thereby an honest pointer, not a duplicate view.
- JSON envelopes gain two additive fields (`spa_route`,
  `web_coverage`); contract version unchanged, old payloads still parse
  via `#[serde(default)]`.
- Docs gain a twelve-row coverage table (covered where vs honestly
  CLI-only with the existing catalog reason); the SPA gains a one-line
  HTML comment noting it is the single interactive surface.
- Contract tests pin every section's deep-link + coverage and the
  header pointer; the catalog integrity suite re-verifies every
  remaining honest-CLI pin still carries its reason.

## BFS Impact Map

- Requirements/scenarios: twelve sections × (deep-link + coverage +
  rendered pointer); each maps to one `PortalSection` arm, one human
  line, two JSON fields, one doc row, one contract-test assertion.
- Modules: `src/portal/model.rs` (methods + two struct fields),
  `src/portal/sections.rs` (populate), `src/portal/render.rs` +
  `src/portal/activity.rs` (human text), `src/portal/portal_tests.rs`
  (literal updates + needles), `tests/portal_contract.rs` (new pins),
  `docs/portal-spa.md` (new), `frontend/index.html` (one comment).
- Contracts/persistence: no Core logic change; no registry write beyond
  the existing single `portal` journal row; no journal shape change.
- Callers/integrations: `forge portal` commands keep their names, args
  and exit codes; CLI/MCP/API/SPA bytes otherwise unchanged.
- Tests/compat: portal + navigation + reference suites green; catalog
  integrity green; live oracle (portal output shows SPA pointers; SPA
  loads with zero JS errors); gate dry-run + bounded full gate.
- Concerns: `.ai-rules/concerns/quality.md` (honest states),
  `.ai-rules/concerns/security.md` (no new surface, no secret/transport
  exposure); architecture (deterministic assembly precedes AI; generated
  projects work without Forge).

## Capabilities

- Local filesystem reads through the existing portal pipeline only.
- Loopback CLI run for the portal text oracle; loopback API+web for the
  SPA zero-JS-error oracle. No external capability.

## Non-goals

- Any SPA feature work: no new view, route, sidebar entry, fetch, or
  dependency; `src/web.rs` allowlist and router untouched.
- Any new API route, write, mutation, provider probe, or adapter run.
- Removing or renaming `forge portal dashboard|view`; exit codes and
  section ids unchanged.
- Rewording any honest-CLI catalog reason that already exists; only a
  missing/empty reason named by the catalog integrity test is fixed.
