# Proposal: independent project inventory and container fleet

## Why

Forge currently consumes a seven-project deployment handoff while
workspace-governance owns a separate 77-project registry. Hard-coding that
relationship makes standalone repositories difficult to deploy and silently
omits projects without a Compose file.

## What Changes

- Define a versioned provider-neutral `ProjectInventory` contract.
- Make local inventory the Forge baseline; workspace-governance becomes an
  optional external adapter selected by explicit configuration.
- Publish every inventory entry that has a valid Compose contract as a Mac
  Docker workload, regardless of whether it is a web app, worker, job, or
  library runtime.
- Register only declared public HTTP services in the Mac port registry and
  route them through Caddy and the existing Cloudflare wildcard tunnel.
- Report missing Compose, invalid inventory, and unavailable adapters
  explicitly instead of filtering projects silently.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-independent-project-inventory-fleet` | Forge publishes an explicit, portable project inventory without requiring workspace-governance | Forge Rust; adapters external | `forge-project-inventory/0.1.0`, Compose/runtime metadata | Existing Forge provider and queue changes | Inventory fixtures and Mac canary/fleet evidence |

This remains one package because inventory normalization, Compose eligibility,
container fleet selection, and public-route registration are one deployment
boundary. OpenPanel and workspace-governance remain consumers/providers, not
Forge dependencies.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge publish providers | `src/publish/providers.rs` | External executable contract and bounded invocation | No inventory contract | Forge | **Extend shared owner** |
| workspace-governance | `deployment/projects.json`, `projects.json`, `scripts/jenkins_manifest.py` | Registry/profile data and optional adapter | Machine root and promotion scope must not become Forge assumptions | workspace-governance | **Adapt through a generic adapter** |
| jenkins-local | `adapters/forge-publish-provider.py` | Mac Docker, runtime overlays, Caddy renderer | Current provider expects one project folder and seven-project handoff | jenkins-local | **Adapt through a generic adapter** |
| Cloudflare/Caddy | `PRODUCTION-RUNBOOK.md`, `generate-caddyfile.py` | One wildcard tunnel and hostname routing | Route registry must distinguish public/private ports | Mac runtime | **Adapt through a generic adapter** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Inventory | New normalized source contract; no fixed `/home/paul/code` or sibling checkout. |
| Fleet | All explicitly inventoried projects are reported; only Compose-ready projects start. |
| Runtime | Mac Docker builds/runs containers; runtime data and cache remain on Mac. |
| Routing | Public HTTP services receive `<project>.tooosall.uk`; internal services remain private. |
| Failure | Missing Compose, missing Dockerfile, invalid revision, unavailable source, and failed health are explicit statuses. |
| Unaffected | OpenPanel implementation, GitHub webhook verification, PostgreSQL data ownership, and Docker cache policy. |

## Capabilities

1. `forge-project-inventory-adapters` — portable local and external inventory sources.
2. `forge-container-fleet-publishing` — all Compose-defined runtime classes.
3. `forge-public-port-routing` — public HTTP port registration behind wildcard Cloudflare ingress.

## Non-goals

- Making workspace-governance a Forge library or runtime prerequisite.
- Creating fake sleeping containers for projects that lack a real runtime.
- Publishing databases, Redis, Jenkins, or private worker ports publicly.
- Creating one Cloudflare route per project.
- Moving source or deployment scripts permanently onto the Mac.
