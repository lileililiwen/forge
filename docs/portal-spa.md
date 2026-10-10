# Portal–SPA convergence

The SPA dashboard served by `forge web` (routes `/projects`,
`/workbench`, `/management`, `/portfolio`, `/delivery`) is the single
interactive browser surface. `forge portal dashboard` and
`forge portal view <section>` render legacy server-side HTML as a
read-only pointer: every section names its SPA deep-link and coverage
verdict (`covered` / `partial` / `cli-only`), and the dashboard header
says so once. No `forge portal` command is removed; section ids, exit
codes and journaling are unchanged.

## Coverage table

| Portal section | SPA deep-link | Coverage | Note |
|---|---|---|---|
| `projects` | `/projects` | covered | Fleet, catalog and reference browsers |
| `features` | `/workbench` | covered | `feature add/remove/upgrade` run in the workbench |
| `components` | `/projects` | covered | Creation-catalog browser |
| `policies` | `/workbench` | partial | Reads in the workbench; policy writes stay in the terminal |
| `specs` | `/projects` | covered | Assurance browser; `spec generate/apply` run in the workbench |
| `agents` | `/projects` | covered | Agent and identity reads |
| `deployments` | `/delivery` | covered | Deploy plan/apply plus release and deploy history |
| `repositories` | `/management` | partial | Workspace onboarding in management; git writes stay in the terminal |
| `documentation` | — (CLI only) | cli-only | Provider translation; run `forge docs translate` in the terminal |
| `analytics` | `/projects` | partial | Metrics read in the assurance browser; live inspect stays in the terminal |
| `servers` | — (CLI only) | cli-only | Loopback transports; run `forge api serve` or `forge web serve` in the terminal |
| `settings` | `/projects` | partial | Governance reads in the assurance browser; `governance use` stays in the terminal |

## Why the CLI-only rows stay CLI-only

Each remaining honest-CLI pin keeps its command-catalog reason, verified
by the catalog integrity suite (`forge_web_command_catalog_contract`:
every non-`web` row carries a non-empty reason and names no route):

- `documentation` (`docs translate`) — provider credentials live in the
  terminal environment, never the browser.
- `servers` (`api serve`, `web serve`) — long-running loopback
  transports, not browser workflows.
- `repositories` git writes (`commit`, `push`, `mirror`) — local Git
  writes with human-initiated confirmation; the browser has no Git
  surface.
- `analytics inspect` — probes live provider planes.
- `settings` writes (`governance use`, portal config) — local state
  transitions with no typed browser route.
- `policies` writes — driftwatch policy application stays terminal-side.

Source: [requirement.md](../requirement.md) §36 (portal sections);
implementation: `src/portal/` (`model.rs` mapping table,
`render.rs`/`activity.rs` pointers); contract pins:
`tests/portal_contract.rs`.
