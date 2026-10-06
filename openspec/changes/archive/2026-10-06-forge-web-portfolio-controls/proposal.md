# Proposal: Manage portfolio metadata and operational evidence in the web

## Why

The project roster does not let operators manage cross-project relationships or understand imported inventory, governance, readiness and provider evidence. These workflows have portfolio-level state and source-owned observations that must remain distinguishable.

## What Changes

- Add browser views/actions for portfolio show, tags, relations, reviews, goals, evidence, sharing/preview/approval, interest and activation readiness.
- Add normalized project catalog queries, gap reports, workspace fleet and portable inventory inspection, governance provider selection/status, analytics inspection and readiness/provider evidence.
- Preserve source ownership, freshness, provenance, privacy thresholds and explicit opt-in for live provider probes.

## Package Boundary and Split Assessment

This is the fourth package after fleet and command catalog. These capabilities share cross-project portfolio and observation models but do not run deployments, publish changes, or perform repository writes; those are the following package.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge portfolio/catalog | `src/registry/portfolio.rs`, `src/main.rs::PortfolioCommands`, project catalog modules | Existing provenance, user-owned metadata and source-owned snapshots | Browser read/write handlers not complete | Forge Core/API/web | **extend shared owner** |
| Fleet/inventory | `src/fleet/mod.rs`, `src/publish/inventory.rs` | Explicit source, freshness, malformed entry and classification contracts | Inputs are CLI-facing and must be projected safely for global admin | Forge | **adapt through a generic adapter** |
| Governance/provider | `src/governance.rs`, `src/main.rs::GovernanceCommands` | Local default and optional bounded external adapters | Live execution needs opt-in and safe status display | Forge provider boundary | **extend shared owner** |
| Workspace Governance | `openspec/specs/workspace-governance-adapter-consumption/spec.md` | Versioned adapter boundary | No direct sibling imports or shared DB allowed | Workspace Governance owns its service; Forge consumes adapter | **adapt through a generic adapter** |

## BFS Impact Map

- **Users:** global operator works across all projects, compares source observations and records Forge-owned portfolio decisions.
- **Data ownership:** Forge-owned tags/goals/reviews remain editable; imported evidence stays append-only/source-owned; provider output remains attributed and bounded.
- **Contracts:** typed authenticated queries/mutations; explicit inventory source and age; readiness/provider statuses remain honest.
- **Privacy/security:** aggregate interest data retains existing thresholds/redaction; live probes require explicit operator action; external adapter paths and local paths are redacted.
- **Failure:** stale/unknown/unavailable/disabled are separate states; malformed source data is reported; partial source results remain visible.
- **Verification:** contract parity, ownership mutation boundaries, threshold tests and provider opt-in tests.
- **Unaffected:** project-file workflow, Git remote writes, publish/deploy and website publication.

## Capabilities

- `forge-web-portfolio-controls`: portfolio-owned metadata management and cross-project evidence views.

Source requirements: `requirement.md` §§32–36; canonical portfolio metadata, interest snapshots, portfolio share/activation, project catalog, fleet observation, portable inventory, governance provider, analytics and readiness specs.

## Non-goals

- No editing source-owned observations as if Forge owned them.
- No automatic provider execution, external account writes, billing or threshold bypass.
- No sibling runtime or database dependency.
