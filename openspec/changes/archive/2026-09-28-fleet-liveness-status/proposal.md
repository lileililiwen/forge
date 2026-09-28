# Proposal: fleet liveness status in the CLI

## Why

`forge deploy status` reports what Forge *published* (journal rows:
per-stage `done`/`failed` and the fleet aggregate), and `docker ps`
reports what the target *runs*, but no CLI surface answers the only
question that matters after a fleet run: **is the service online?**
Today that gap is filled by ad-hoc `ssh` + `curl` one-liners whose
output is untyped, unredacted, and incomparable across runs. Live
evidence from the `decoupled-remote-publish` rollout proved the need:
deploys reported `healthy=true` while apps crash-looped behind
missing secrets (`alethefy` API-key refusal), Caddy served
`Unknown application hostname` for unrouted hosts, and Cloudflare
returned `502` for rules whose upstream was absent — none of which
the journal distinguishes.

## What Changes

- Add a read-only `forge fleet online` command that joins three live
  facts per roster project and renders one typed verdict:
  - **container**: the target's `docker ps` names+status over the
    existing SSH transport (no new access path);
  - **route**: the host rules parsed from the *served*
    `platform/Caddyfile` (ground truth of what the router matches,
    not a local projection);
  - **reachability**: one bounded HTTPS GET per routed host with a
    bounded body read.
- Verdicts: `ONLINE` (the app answered with anything other than the
  router fallback or a gateway error), `DOWN` (route exists but the
  origin is unreachable or answers 502/503/504), `NO-ROUTE` (no
  router rule for the project), `NOT-DEPLOYED` (no container on the
  target). Unknown-hostname fallback bodies never read as online.
- Emit a versioned `forge-fleet-liveness/0.1.0` JSON document plus a
  human table; non-zero exit unless every probed host is online.
  Probes never send secrets, never write the journal, registry, or
  target, and every captured string passes
  `policy::redact_credentials`.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `fleet-liveness-status` (Forge Rust) | One CLI command proves per-project online state with typed verdicts | Forge Rust, same crate; target owns Docker + file tree + router | `forge-fleet-liveness/0.1.0` report; roster in via `forge-project-inventory/0.1.0`, routes out of served Caddyfile | `decoupled-remote-publish`, `forge-independent-project-inventory-fleet`, `forge-publish-queue-status` | Unit tests + `fleet online --dry-run` shape + live Mac run → verdicts |

Single outcome, single owner (Forge Rust), single lifecycle
(read-only probe), single oracle (live verdicts). Split signals
considered: the later web UI consumes this report but ships its own
route/lifecycle (separate `portal-web-ui` package, depends on this
one); remediation/redeploy stays in `forge publish fleet` and is an
explicit non-goal. This is the smallest independently verifiable
unit: it can be implemented and tested without the portal.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `forge deploy status` journal | `src/main.rs:cmd_deploy_status`, `forge-publish-queue-status` spec | Queue/phase journal rows, `--watch` polling shape | Records publish outcomes, never probes liveness | Forge | **Keep local** (this package joins journal rows as one input, does not redefine them) |
| Publish SSH transport | `src/publish/mod.rs:SubprocessTransport`, `remote_compose.rs:docker_command` | Bounded `ssh`/`docker` argv, `/usr/local/bin/docker` + Desktop PATH, credential redaction | Built for publish stages, not probes | Forge | **Extend shared owner** (reuse transport + redaction for `docker ps` / `cat Caddyfile` probes) |
| Served Caddyfile | `src/publish/caddy.rs:choose_route`, target `platform/Caddyfile` | Route vocabulary (`http://<project>.<domain>`, nav host, unknown-hostname fallback) | Renderer projects; liveness needs the served ground truth | Forge | **Keep local** (parse served rules; never re-derive routing) |
| `curl` subprocess | None in tree (no HTTP client in `Cargo.toml`) | Bounded-subprocess precedent (`run_subprocess`, policy probes) | No shared HTTP runner exists | Forge | **Keep local** (small bounded `curl -s -m` runner; `ureq` rejected to keep the dependency closure unchanged per `cargo deny`) |
| Portal dashboard | `control-plane-portal` spec | Twelve-section read model, worst-status rollup | No liveness inputs exist yet | Forge | **Consumer** (`portal-web-ui` reads this report; this package does not touch portal code) |

## BFS Impact Map

| Surface | Impact |
|---|---|
| CLI | New `fleet online` subcommand (+ `--inventory`/`--fleet-registry` roster flags mirroring `publish fleet`, `--format`, `--timeout-secs`); no existing flag changes |
| Core | New read-only module (liveness join + verdicts); no registry columns, no journal writes, no new journal kind |
| Target | Read-only: `ssh cat Caddyfile`, `docker ps`, HTTPS GETs; secret files never read, secret values never rendered |
| Contracts | New `forge-fleet-liveness/0.1.0`; existing inventory/queue/Caddy contracts unchanged |
| MCP/API/portal | Untouched (portal-web-ui consumes the report later) |
| Failure surface | Missing `curl`/`ssh`/`docker`, target timeouts, and malformed Caddyfile each surface as typed `unavailable` verdicts, never fabricated online |
| Tests | Unit (verdict classifier, Caddyfile host parser, redaction) + CLI contract (fixture roster + stubbed probes) + live Mac evidence |

## Capabilities

### New Capabilities

- `fleet-liveness-status`: read-only per-project online verdicts joining container state, served router rules, and HTTP reachability.

### Modified Capabilities

(none — `forge-publish-queue-status`, `forge-independent-project-inventory-fleet`, and `fleet-registry-observation` keep their SHALLs)

## Non-goals

- Remediation, redeploy, restart, or any mutation of the target, registry, or journal — `forge publish fleet` owns recovery.
- Alerting, history, flapping detection, or uptime SLAs.
- Probing non-HTTP ports or inventing health endpoints inside sibling projects.
- MCP/API exposure in this package (a later consumer may project the report).
