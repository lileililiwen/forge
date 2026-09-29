# Proposal: One reviewed project-to-production workflow

## Why

Forge already has project generation, quality evidence, a publish journal, and explicit publish providers. OpenPanel has an active `forge-publish-provider/0.1.0` change, while Hermora manages post-publish site operations. The remaining user gap is coordinating these stages and their manual handoffs as one visible workflow.

## What Changes

- Add a project delivery view that summarizes source revision, required checks, provider preflight, stage publish, health, production approval, and optional Hermora onboarding.
- Reuse Forge's publish journal and provider protocol; OpenPanel remains the deployment/rollback owner.
- Preserve explicit human confirmation before source push and production promotion.
- Allow a retry of failed Hermora onboarding without repeating a successful deployment.

## Package Boundary and Split Assessment

This package orchestrates existing owners; it does not implement their underlying state machines. OpenPanel provider implementation, GitHub CLI repository work, and Hermora enrollment each have independent owners and tests, so they are separate dependencies.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-publish-provider` | Expose OpenPanel's current delivery lifecycle to Forge | OpenPanel, Rust | `forge-publish-provider/0.1.0` | OpenPanel application delivery | provider conformance |
| `published-site-onboarding` | Enroll deployed app in Hermora | Hermora, C#/.NET | scoped onboarding API | Hermora Management API | authenticated idempotent registration |
| `github-cli-project-workflows` | Use current `gh` session for repo setup | Forge, Rust | bounded local CLI adapter | installed `gh` | fake process plus read-only auth smoke |
| `project-to-production-workflow` | Show and advance one project delivery lifecycle | Forge, Rust | existing journal and provider contracts | the three packages above | stage-to-production contract journey |

Order: OpenPanel provider and Hermora onboarding contracts → Forge delivery workflow. Existing OpenPanel `forge-publish-provider` is reused as-is; this package does not rewrite its pending change.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge publish subsystem | `src/publish/`, `forge publish` CLI, existing `/ui` route | operation IDs, dry-run, provider invocation, journal, confirmation | no single project lifecycle view across preflight/stage/health/promotion | Forge | **extend shared owner** |
| OpenPanel | provider contract and existing application-delivery | release digest, health, rollback | provider facade remains OpenPanel-owned | OpenPanel | **adopt** |
| Hermora | site registry, `CredentialRef`, Management API and advanced operations | independent site management and SEO/content operations | no deployment-triggered enrollment contract | Hermora | **adapt through a generic adapter** |
| Workspace Governance | runtime templates and project declarations | runtime metadata and reusable Compose | not a publish workflow state machine | Workspace Governance | **adopt** as read-only project metadata |

## BFS Impact Map

- **State:** `draft → preflighted → awaiting-stage-confirmation → staging → stage-healthy|failed → awaiting-production-approval → production → healthy|degraded → hermora-pending|connected`.
- **Persistence:** existing Forge project and publish journals; append bounded delivery evidence with provider, revision/digest, environment, timestamps, and typed status.
- **Ordering:** validate source revision and Gates; preflight provider; explicitly publish staging; verify health; require separate production approval; invoke Hermora onboarding only after confirmed successful deployment.
- **Failure:** never infer success from process exit; failed health prevents promotion; provider timeout is unknown until queried; Hermora failure leaves deployment success intact and offers idempotent onboarding retry.
- **Security:** per-project authorization, redacted bounded logs, no plaintext secrets, private repository default, no implicit push or production deployment.
- **Compatibility:** existing `forge publish` commands and provider contract remain usable; new UI/API is an additive consumer.

## Capabilities

- **New:** `project-to-production-workflow`.
- **Modified:** `publish-observability`, `control-plane-portal` with additive delivery summaries only.

## Non-goals

- No deployment executor inside Forge and no duplicate OpenPanel/Hermora state machine.
- No autonomous production promotion, domain/DNS purchase, billing, or rollback without confirmation.
- No required Hermora dependency; unmanaged sites remain valid.
- No local Mac or Docker Desktop requirement.
