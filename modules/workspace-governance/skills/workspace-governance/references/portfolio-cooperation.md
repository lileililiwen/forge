# Portfolio cooperation

Use this route when a request spans Forge, a shared/common project, or a
manager's handoff to another owner. It is a routing aid, not a second source of
architecture policy. Read the named owner's current README, HANDOFF, ROADMAP,
canonical OpenSpec spec, and implementation before asserting that a handoff is
available or complete. A project's planned package, archived spec, and live
provider result are different evidence states.

## Ownership map

| Concern | Owning project | Handoff |
| --- | --- | --- |
| Project declarations, workspace discovery, governance vocabulary, capability instances, deployment metadata | Workspace Governance | Read-only governance provider and versioned declarations for Forge |
| Project creation, deterministic assets, local catalog, review, developer workflow, and coordination | Forge | Owns the human-facing workflow and consumes provider contracts; generated projects own their source and native verification |
| Validated idea evidence and explicit export | Hypora | Privacy-bounded idea-graduation snapshot to Forge through platform-contracts |
| Language-neutral wire schemas and fixtures, including capability entry shapes | platform-contracts | Pinned versioned contracts for producers and consumers; Workspace Governance owns registered capability instances |
| Reusable language-specific code | dotnet-platform-libs and rust-platform-libs | Versioned packages adopted through pinned consumer manifests after publication is verified |
| Quality policy execution and Gate evidence | Driftwatchdog | Versioned evidence export consumed by Forge; Forge records the invocation and revision-bound observation |
| Deployment execution, health, promotion, and rollback | OpenPanel, with Jenkins/Mac execution where configured | Forge coordinates explicit publish stages through OpenPanel's provider contract |
| Published-site enrollment and post-publish operations | Hermora | Separate, retryable enrollment after a successful deployment record |
| Product behavior, source, tests, and local runtime proof | Each product repository | Native build/test and real runtime evidence remain with that product |

The current portfolio decisions, including the internal SSO boundary, agent
runtime split, and capability-registry split, live in `decisions.json` and
`docs/toolchain-decisions.md`. For provider adoption use
`docs/capabilities.md` and `docs/adoption-grant.md`; for declarations and
release evidence use the other routes in the front-door skill.

Load the owner's installed skill for the next step: forge-project-workflow for
project lifecycle and coordination, hypora-idea-graduation for approved idea
export, platform-contract-consumption for wire schemas,
dotnet-platform-adoption or rust-platform-adoption for language consumers,
driftwatchdog-gate for verification evidence, openpanel-forge-delivery for
publish operations, and hermora-site-onboarding for post-publish enrollment.

## Follow the handoff

1. Identify the operation and its owner in the table. Inspect that owner's
   current contract, status, and evidence before choosing a command or change.
2. For idea graduation, require Hypora's approved aggregate-only export and a
   pinned platform contract; Forge validates and previews the local snapshot
   before the user confirms creation. Do not transfer leads, raw answers,
   sessions, events, or traffic-source data.
3. For project delivery, use Forge's reviewed create/refine/preview and
   repository workflow, then OpenPanel's staged deployment, health check, and
   explicit promotion. Treat Hermora enrollment as a separate retryable step.
4. For Gate or governance findings, consume the owning provider's versioned
   observation. Preserve revision, provenance, failure and unavailable states;
   neither Forge nor a README can turn an unverified provider result into PASS.
5. Change the owning repository first. Use a pinned contract at the consumer
   boundary, verify producer and consumer behavior, and record separate
   evidence for each repository. Do not add source-level sibling dependencies
   or silently edit another project's source.

Start with Forge's current architecture and roadmap for its implemented
surface, and the provider's current files for the provider side. The Forge
roadmap's archived packages do not by themselves establish a live deployment,
provider integration, or product runtime result.
