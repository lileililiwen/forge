# Proposal: Add project setup, planning and quality workflows to the web

## Why

An inventory-only page cannot maintain projects. Operators need browser workflows for project onboarding, inspection, features, planning, agents, standards and quality while using the same deterministic Core behavior as CLI/MCP.

## What Changes

- Add project-scoped pages/actions for list, inspect, register/import/new, profile/kit, doctor/check/test/gate, project identity sessions, feature/upgrade, spec/agent, component/UI-pattern/intent/procedure, standard/remediation and semantic description/classification.
- Reuse typed Core operations and existing contracts; show plans and diffs before writes, explicit confirmation for writes, operation IDs and journal evidence.
- Keep source-only, interactive-terminal and unsupported operations visibly CLI-only in the command catalog.

## Package Boundary and Split Assessment

This is the third package, after fleet and catalog. The workflows share project identity/context and local project file operations. Portfolio metadata and remote delivery have different state and authorization lifecycles and are separate packages.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge Core and CLI | `src/main.rs` dispatch; modules under `src/` | Existing typed operations, plans, validators and journals | Some flows are only CLI-wired or require a TTY | Forge Core/API/web release | **extend shared owner** |
| Control-plane portal contract | `openspec/specs/control-plane-portal/spec.md` | Project detail, accessible operations, partial results and tracking expectations | Current implemented browser has only global roster/login | Forge portal | **extend shared owner** |
| AllTools login | supplied reference HTML | Sign-in composition only | No project workflow contracts | Independent AllTools project | **keep local** |

## BFS Impact Map

- **Actors/flow:** operator chooses a managed project, reviews evidence, plans, confirms and tracks a workflow.
- **Contracts:** typed project JSON endpoints, stable operation IDs, plan/diff/confirm model, honest capability states.
- **Persistence:** existing Forge registry/journal and project-owned files; no duplicate workflow database.
- **Security:** global session plus per-action server-side authorization and project path scoping; no generic shell endpoint.
- **Failure:** conflict, stale evidence, missing project, provider unavailable, partial operation and validation errors remain visible and recoverable.
- **Verification:** parity against Core results, forbidden cross-project paths, dry-run no-write assertions, browser keyboard/error/confirmation flows.
- **Unaffected:** portfolio-owned tags/goals, direct remote writes and publish/deploy orchestration.

## Capabilities

- `forge-web-project-workbench`: project-scoped setup, planning and quality operations through browser workflows.

Source requirements: `requirement.md` §§34–36; canonical `control-plane-portal`, `project-to-production-workflow`, `standard-pack-registry-and-snapshots`, and relevant command specs.

## Non-goals

- No arbitrary shell/SQL/CLI-text execution.
- No mutation of external provider state beyond a separately specified and authorized typed Core operation.
- No replacement of the terminal for commands that fundamentally require an interactive TTY.
