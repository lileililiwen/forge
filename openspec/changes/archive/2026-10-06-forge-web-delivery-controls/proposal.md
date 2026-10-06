# Proposal: Add confirmed repository and delivery workflows to the web

## Why

Forge's remaining major command families create commits, push refs, publish sites, deploy services, release artifacts or contact external providers. They cannot safely be represented as simple links or buttons; operators need plans, exact confirmation, progress and recovery in the web UI.

## What Changes

- Add web workflows for commit/push/mirror, documentation translation, release, deploy, publish, staged delivery, GitHub metadata operations and Studio preview/refinement.
- Reuse existing Core plans, provider boundaries, allowlists and journals; show target, files/ref, external effects and rollback/recovery before confirmation.
- Classify MCP/API/web server setup, low-level contract emission/validation and other transport/build commands as CLI-only where browser execution is not a product workflow.

## Package Boundary and Split Assessment

This is the final package in the proposed web queue. These workflows share remote/external side effects, plans, explicit consent and operation tracking. They depend on project identity/workbench and portfolio approval state, but do not redefine those contracts.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge delivery Core | `src/main.rs::DeliveryCommands`, deploy/publish/release modules | Existing plans, providers, state machines, journal and rollback contracts | Browser operation route and confirmation review need implementation | Forge Rust/API/web | **extend shared owner** |
| OpenPanel | canonical `project-to-production-workflow` references and `openpanel-forge-delivery` contract | Independent deployment lifecycle/provider boundary | Forge must consume typed handoff/artifacts, not call sibling runtime/source | OpenPanel owns deployment control plane | **adapt through a generic adapter** |
| Hermora | `hermora-site-onboarding` contract | Explicit post-publication management capability boundary | Site-management operations only after successful published site enrollment | Hermora owns site management | **adapt through a generic adapter** |
| AllTools | user-provided reference | Visual-only input | Unrelated delivery contracts | Independent project | **keep local** |

## BFS Impact Map

- **Actors/flow:** operator selects a project, reviews a plan and external effects, confirms, tracks operation, then receives success/failure/recovery.
- **State/contracts:** typed operation resources, confirmation digest, idempotency, journal/event timeline, provider attribution.
- **Integrations:** Git, deploy/publish adapters, GitHub, mirrors, translation providers, release and Studio preview remain existing bounded adapters.
- **Security:** remote writes require explicit confirmation bound to target and plan; credentials remain in configured providers; global session never substitutes provider credentials.
- **Failure:** timeout, partial remote write, rollback, provider unavailable and ambiguous outcome are recorded and require reconciliation rather than blind retry.
- **Verification:** dry-run/no-write, confirmation mismatch, provider sandbox, idempotent operation and partial-failure/recovery tests.
- **Unaffected:** CLI transports continue working; no deployment infra is provisioned by the web change.

## Capabilities

- `forge-web-delivery-controls`: safely review, confirm and track existing repository and delivery operations from the browser.

Source requirements: `requirement.md` §§34–36; canonical `project-to-production-workflow`, publish provider, release, deployment, mirror, docs, Studio, OpenPanel and Hermora contracts.

## Non-goals

- No automatic pushes, publishing, deploys, releases, translation or external writes.
- No provider credentials displayed in browser or copied from global login credentials.
- No generic shell, raw Git command, arbitrary URL fetch or sibling runtime call.
- No new infrastructure deployment or external publication authorization.
