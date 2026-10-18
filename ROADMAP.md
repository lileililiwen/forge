# Forge roadmap

Status: 24 baseline entries plus five audit/foundation follow-ups are implemented and archived, and all ten sibling-integration packages (orders 30 through 39 — `driftwatch-cli-alignment`, `external-checker-emission`, `workspace-governance-adapter-consumption`, `fleet-registry-observation`, `supervised-agent-adapters`, `jenkins-deploy-adapter-consumption`, `workspace-metadata-emission`, `gate-runtime-evidence`, `gate-evidence-export`, and `gate-evidence-export-consumption`) plus the independent inventory and container fleet package (`forge-independent-project-inventory-fleet`) and the later publish, portal, portfolio, catalog and studio packages listed under [Later packages](#later-packages) are implemented, archived and promoted to their canonical specs — 154 archived changes and 112 canonical specs. The completed source consolidation now proceeds to Forge's Quality portal UX. Sibling portability is deferred; remaining packages are dependency-ordered in the consolidation section below. The follow-ups address runtime/test hardening, native/release evidence, real provider evidence, specification governance, standalone governance-provider switching, read-only fleet observation from the workspace registry, supervised agent-runtime delegation, the executable Jenkins deploy executor boundary, sibling-compatible workspace metadata at generation, the real Driftwatchdog CLI surface for the policy plane, Driftwatchdog's evidence-export verb consumed by Forge, and the portable inventory contract that decouples the Forge fleet from workspace-governance. Versions are delivery targets, not release promises. [requirement.md](requirement.md) is authoritative; [coverage](docs/requirements-coverage.md) accounts for every numbered section.

## Scope (as delivered)

The v0.1 boundary is exactly the five initial CLI commands, versioned manifest, local project registry and five MVP profiles. Feature installation, AI planning, orchestration, portal, translation, release, SSO and distributed deployment are excluded from v0.1. Local doctor inspection is distinct from the v0.3 DriftWatch integration.

v0.2 delivers feature contracts and deterministic lifecycle support, project/fleet upgrades and the additional React profile. Catalog examples do not imply every feature is implemented across every profile. v0.3 reuses quality/spec/agent infrastructure. v0.4 exposes only verified stable operations via MCP. v0.5 adds explicit remote lifecycle operations.

Later packages preserve the complete product vision without making it an MVP prerequisite. Fine-grained components and UI patterns precede the advanced Intent planner; API precedes the portal. Standard OIDC integration does not create an identity provider.

## Dependency order

Follow the order below by default; every dependency was implemented and verified before its consumer started. Dependencies name archived change IDs; use the promoted canonical specs and archived evidence.

| Order | Target | Change | Prerequisites | Brief sections |
| --- | --- | --- | --- | --- |
| 1 | 0.1 | [core-manifest-registry](openspec/specs/core-manifest-registry/spec.md) | None | §1, §2, §3, §4, §5, §6, §7, §34, §37, §38, §39, §44, §45, §47 |
| 2 | 0.1 | [profile-registry](openspec/specs/profile-registry/spec.md) | `core-manifest-registry` | §3, §9, §37, §39 |
| 3 | 0.1 | [project-import](openspec/specs/project-import/spec.md) | `core-manifest-registry`, `profile-registry` | §8, §34, §39 |
| 4 | 0.1 | [deterministic-project-generation](openspec/specs/deterministic-project-generation/spec.md) | `profile-registry`, `project-import` | §3, §14, §15, §34, §39 |
| 5 | 0.1 | [doctor-maturity-assessment](openspec/specs/doctor-maturity-assessment/spec.md) | `deterministic-project-generation` | §23, §25, §34, §39 |
| 6 | 0.2 | [feature-lifecycle](openspec/specs/feature-lifecycle/spec.md) | `doctor-maturity-assessment` | §10, §14, §15, §34, §40 |
| 7 | 0.2 | [project-upgrade-orchestration](openspec/specs/project-upgrade-orchestration/spec.md) | `feature-lifecycle` | §26, §34, §40 |
| 8 | 0.2 | [extended-profile-catalog](openspec/specs/extended-profile-catalog/spec.md) | `feature-lifecycle` | §9, §13 |
| 9 | 0.3 | [quality-policy-integration](openspec/specs/quality-policy-integration/spec.md) | `doctor-maturity-assessment`, `feature-lifecycle` | §24, §32, §41 |
| 10 | 0.3 | [specification-remediation](openspec/specs/specification-remediation/spec.md) | `project-upgrade-orchestration`, `quality-policy-integration` | §22, §26, §32, §34, §41 |
| 11 | 0.3 | [agent-runtime-workflows](openspec/specs/agent-runtime-workflows/spec.md) | `specification-remediation` | §21, §22, §32, §34, §41 |
| 12 | 0.4 | [mature-mcp-surface](openspec/specs/mature-mcp-surface/spec.md) | `agent-runtime-workflows` | §20, §34, §42 |
| 13 | 0.5 | [repository-distribution](openspec/specs/repository-distribution/spec.md) | `agent-runtime-workflows` | §27, §32, §34, §43 |
| 14 | 0.5 | [documentation-translation](openspec/specs/documentation-translation/spec.md) | `core-manifest-registry` | §28, §34, §43 |
| 15 | 0.5 | [release-publishing](openspec/specs/release-publishing/spec.md) | `repository-distribution`, `documentation-translation`, `quality-policy-integration` | §29, §34, §43 |
| 16 | 0.5 | [adapter-deployment](openspec/specs/adapter-deployment/spec.md) | `release-publishing` | §30, §34, §43 |
| 17 | later | [semantic-component-registry](openspec/specs/semantic-component-registry/spec.md) | `feature-lifecycle`, `project-upgrade-orchestration` | §3, §11, §12, §45, §47 |
| 18 | later | [semantic-ui-patterns](openspec/specs/semantic-ui-patterns/spec.md) | `semantic-component-registry`, `extended-profile-catalog` | §13, §46, §47 |
| 19 | later | [validated-intent-planner](openspec/specs/validated-intent-planner/spec.md) | `semantic-component-registry`, `semantic-ui-patterns`, `quality-policy-integration` | §15, §16, §17, §18, §45, §46 |
| 20 | later | [ai-procedure-skills](openspec/specs/ai-procedure-skills/spec.md) | `mature-mcp-surface`, `validated-intent-planner`, `adapter-deployment` | §19, §32 |
| 21 | later | [central-admin-identity](openspec/specs/central-admin-identity/spec.md) | `feature-lifecycle`, `adapter-deployment` | §25, §31 |
| 22 | later | [external-planes-analytics](openspec/specs/external-planes-analytics/spec.md) | `agent-runtime-workflows`, `adapter-deployment`, `repository-distribution` | §32, §33 |
| 23 | later | [core-http-api](openspec/specs/core-http-api/spec.md) | `mature-mcp-surface`, `adapter-deployment`, `external-planes-analytics` | §35 |
| 24 | later | [control-plane-portal](openspec/specs/control-plane-portal/spec.md) | `core-http-api`, `external-planes-analytics`, `semantic-ui-patterns` | §36 |
| 25 | audit | `runtime-hardening-and-test-isolation` | `control-plane-portal` | cross-cutting failure boundaries |
| 26 | audit | `profile-and-release-readiness` | `runtime-hardening-and-test-isolation` | §9, §15, §29, §34, §39, §43 |
| 27 | audit | `provider-integration-evidence` | `profile-and-release-readiness` | §24, §30, §31, §33, §43 |
| 28 | audit | `specification-governance-refresh` | `runtime-hardening-and-test-isolation`, `profile-and-release-readiness`, `provider-integration-evidence` | repository evidence governance |
| 29 | audit | `governance-provider-contract-and-local-default` | `core-manifest-registry`, `provider-integration-evidence`, `control-plane-portal` | standalone optional governance provider boundary |
| 30 | sibling | [driftwatch-cli-alignment](openspec/specs/quality-policy-integration/spec.md) | `quality-policy-integration`; driftwatchdog `checker-machine-output` (external, archived 2026-09-24) | §23, §24, §32, §41 |
| 31 | sibling | [external-checker-emission](openspec/specs/external-checker-emission/spec.md) | `doctor-maturity-assessment`, `quality-policy-integration` | §23, §24, §32, §34 |
| 32 | sibling | [governance-provider-contract](openspec/specs/governance-provider-contract/spec.md) | `governance-provider-contract`; workspace-governance `forge-governance-adapter` (external, archived 2026-09-24) | §32, §38 |
| 33 | sibling | [fleet-registry-observation](openspec/specs/fleet-registry-observation/spec.md) | `control-plane-portal` | §7, §32, §36 |
| 34 | sibling | [supervised-agent-adapters](openspec/specs/agent-runtime-workflows/spec.md) | `agent-runtime-workflows` | §21, §22, §32 |
| 35 | sibling | [jenkins-deploy-adapter-consumption](openspec/specs/adapter-deployment/spec.md) | `adapter-deployment`, `provider-integration-evidence` | §30, §32, §43 |
| 36 | sibling | [deterministic-project-generation](openspec/specs/deterministic-project-generation/spec.md) | `deterministic-project-generation` | §14, §32 |
| 37 | sibling | [gate-runtime-evidence](openspec/specs/gate-runtime-evidence/spec.md) | `driftwatch-cli-alignment`, `profile-and-release-readiness` | §24, §25, §29, §32 |
| 38 | sibling | [driftwatchdog gate-evidence-export](openspec/specs/gate-runtime-evidence/spec.md) | `gate-runtime-evidence`; driftwatchdog `gate-evidence-export` (external, archived 2026-09-27) | evidence export entry point |
| 39 | sibling | `gate-evidence-export-consumption` | `driftwatchdog gate-evidence-export` (archived sibling) | §24, §25, §29, §32, §38 |
| 40 | later | [forge-independent-project-inventory-fleet](openspec/specs/forge-independent-project-inventory-fleet/spec.md) | `forge-publish-queue-status`, `forge-publish-observability-revision-containers` | §7, §23, §29, §32, §34, §36 |

## Recently delivered

`project-metadata-bundle` was archived as `2026-10-17-project-metadata-bundle`.
Forge now builds a deterministic, receipt-protected project metadata document
from declared `.project.json` values, optional `forge.yaml`, documentation
links and canonical specification identifiers. Registered local projects can
be searched through the CLI and the read-only MCP tool
`search_project_metadata`; the closed fifteen-key project catalog contract and
the sibling CodeGraph boundary remain unchanged. This work follows
`interface-declaration-consumption`, implemented on 2026-10-17.

## Later packages

Delivered after the dependency-order table above and archived with their
canonical specs. The publish chain runs `forge-publish-plugin-orchestration` →
`sibling-cwd-publish` → `decoupled-remote-publish` → `fleet-live-rollout` /
`fleet-liveness-status`; the browser surfaces sit on `portal-web-ui`; the
catalog chain runs `project-catalog-query-contract` →
`project-evidence-gap-assessment` → `project-local-remediation-plans` /
`project-semantic-description-review` / `project-query-consumer-surfaces`.

### Proposed authenticated web command center

The active `forge-global-admin-portal` change supplies global sign-in and the
initial dashboard. After that change is implemented and archived, continue in
this dependency order; each package has independent requirements and an
acceptance oracle:

1. `forge-web-project-fleet` — combine the local registry and explicitly
   selected inventory sources; always include Forge itself.
2. `forge-web-command-catalog` — enumerate every top-level and nested Rust CLI
   command and map it to a web workflow or a reasoned CLI-only state.
3. `forge-web-project-workbench` — project setup, identity, planning and quality
   workflows, depending on fleet and command catalog.
4. `forge-web-portfolio-controls` — cross-project metadata and evidence views,
   depending on fleet and command catalog.
5. `forge-web-delivery-controls` — confirmed repository, provider and delivery
   operations, depending on command catalog, workbench and portfolio controls.

These were planning packages; the five web command-center packages and nine
web-gap browsers (command reference, portfolio completion, project catalog,
release/deploy history, agent/identity reads, creation catalog, assurance,
shipping/provider reads, portal–SPA convergence) plus `workbench-health-latency`
are implemented, archived and promoted — see [portal–SPA](docs/portal-spa.md).
The web frontend remains standalone HTML/CSS/JavaScript served by Forge's Rust web listener;
the API remains JSON-only. Every CLI command stays discoverable, including
transport/build commands that receive explicit CLI-only guidance rather than
unsafe generic shell execution.

- **Publish and release:** [platform-contract-consumption](openspec/specs/platform-contract-consumption/spec.md), [artifact-and-ci-baseline](openspec/specs/artifact-and-ci-baseline/spec.md), [publish-commands](openspec/specs/publish-commands/spec.md), [forge-publish-plugin-orchestration](openspec/specs/forge-publish-plugin-orchestration/spec.md), [sibling-cwd-publish](openspec/specs/sibling-cwd-publish/spec.md), [forge-publish-queue-status](openspec/specs/forge-publish-queue-status/spec.md), [forge-publish-observability-revision-containers](openspec/specs/forge-publish-observability-revision-containers/spec.md), [decoupled-remote-publish](openspec/specs/decoupled-remote-publish/spec.md), [fleet-live-rollout](openspec/specs/fleet-live-rollout/spec.md), [fleet-liveness-status](openspec/specs/fleet-liveness-status/spec.md).
- **Portal, portfolio and delivery:** [portal-web-ui](openspec/specs/portal-web-ui/spec.md) (promoting the `portal-browser-sign-in` OIDC sign-in and `portal-accessible-responsive-ui` responsive/accessibility work), [portfolio-metadata-and-review](openspec/specs/portfolio-metadata-and-review/spec.md), [interest-snapshots](openspec/specs/interest-snapshots/spec.md), [portfolio-share](openspec/specs/portfolio-share/spec.md), [portfolio-activation-readiness](openspec/specs/portfolio-activation-readiness/spec.md), [project-to-production-workflow](openspec/specs/project-to-production-workflow/spec.md), [hypora-graduation-import](openspec/specs/hypora-graduation-import/spec.md).
- **Catalog, remediation and provider:** [project-catalog-query-contract](openspec/specs/project-catalog-query-contract/spec.md), [project-evidence-gap-assessment](openspec/specs/project-evidence-gap-assessment/spec.md), [project-local-remediation-plans](openspec/specs/project-local-remediation-plans/spec.md), [project-query-consumer-surfaces](openspec/specs/project-query-consumer-surfaces/spec.md), [project-semantic-description-review](openspec/specs/project-semantic-description-review/spec.md), [github-project-metadata-adapter](openspec/specs/github-project-metadata-adapter/spec.md), [github-cli-project-workflows](openspec/specs/github-cli-project-workflows/spec.md).
- **Platform, profiles and governance:** [governance-provider-contract](openspec/specs/governance-provider-contract/spec.md) (including the archived `governance-vocabulary-consumption` requirement), [profile-and-release-readiness](openspec/specs/profile-and-release-readiness/spec.md) (including `profile-matrix-evidence`), [standard-pack-registry-and-snapshots](openspec/specs/standard-pack-registry-and-snapshots/spec.md), [site-studio-preview-refinement](openspec/specs/site-studio-preview-refinement/spec.md), [react-web-live-preview](openspec/specs/react-web-live-preview/spec.md), [readiness](openspec/specs/readiness/spec.md) (the archived `post-mvp-readiness` review).

## Release acceptance

- **v0.1:** all five MVP profile fixtures generate and build using native toolchains without Forge; import preserves existing source; registry persists; doctor distinguishes missing, inapplicable and unverified evidence. Missing toolchains block a claim of verified profile support.
- **v0.2:** tested feature/profile support matrix, safe add/remove/upgrade, repeatable migrations and isolated fleet failures. Unsupported catalog entries stay explicit.
- **v0.3:** real DriftWatch and agent-manager contract evidence, traceable spec remediation, tests and scoped Git operations. Fakes alone are insufficient integration evidence.
- **v0.4:** transport parity and negative authorization/input tests for every exposed mature tool.
- **v0.5:** provider sandbox or controlled integration evidence for mirrors, translations, release stages and deployment health; partial external outcomes remain partial.
- **Later:** each bounded package was verified independently; no portal/planner/SSO claim follows from the earlier CLI milestones alone — each has its own archived evidence.

## Deferred choices

The brief recommends Rust Core/CLI, SQLite initially, YAML and stdio MCP. Versions and persistence approach are recorded in the foundation ADR; product contracts are not bound to Rust. HTTP authentication and remote exposure were decided in their own changes. The portal ships as a read-only Core/CLI data surface (`forge portal dashboard|view`); a graphical portal framework (ASP.NET Core / Next.js) remains a deferred downstream choice. PostgreSQL registry migration, specialist profiles, Blazor UI adapters and additional Git providers remain conditional future candidates. No unsupported runtime command is represented as implemented.

Product-native monetization for one selected product remains a deferred, product-owned candidate: `portfolio-interest-snapshots` stores `paid_interest_events` as an aggregate signal rather than a payment record, `.project.json` declares `billing: blocked`, and the Forge-side gate that decides when the handoff is justified is delivered by the archived `portfolio-activation-readiness` change and its canonical [portfolio-activation-readiness](openspec/specs/portfolio-activation-readiness/spec.md) spec. No portfolio-wide billing is proposed, and Forge gains no price, plan, subscription, entitlement, checkout or revenue-attribution surface.

Live analytics aggregate evidence for the interest store is likewise a provider-owned prerequisite, not missing Forge work: the import verb and its store are complete, and what is absent is a producer that emits `forge-portfolio-interest/0.1.0` documents. It is registered with its exact next action in [docs/provider-evidence.md](docs/provider-evidence.md).

## Forge source consolidation and retirement

Forge is the developer-facing owner and facade. Components can live in `src/`
or ordinary subdirectories/workspaces and may keep independent build and
publication boundaries. They are not Git submodules, and generated projects
must not depend on sibling checkouts. Migration proceeds in this order.
Workspace Governance's original source repository is already archived; row 9
improves the maintained Forge copy independently of that archive status. The
final row applies to origins that have not yet been archived:

| Order | Package | Scope / completion evidence | State |
| --- | --- | --- | --- |
| 1 | `forge-ci-precommit-templates` | Versioned baseline templates; compatible new-project default; legacy install/upgrade; same Forge Gate in local hook and CI | Complete; archived 2026-10-17 |
| 2 | `forge-driftwatchdog-test-reconciliation` | Reconcile in-tree DriftWatchdog Gate tests with Forge's authoritative behavior; the previously failing 10 tests pass in the serial workspace suite | Complete; archived 2026-10-18 |
| 3 | `forge-portable-binary-distribution` | Publish versioned, checksummed Forge binaries so generated CI can install a pinned release without a source checkout | Complete; archived 2026-10-18 |
| 4 | `forge-runtime-template-internalization` | Move the versioned runtime template registry, renderer and assets into Forge; keep template IDs and `.project.json` behavior compatible; package assets with Forge | Complete; archived 2026-10-18 |
| 5 | `forge-workspace-governance-in-tree` | Import the complete Workspace Governance project and its six local-only commits as an ordinary in-tree project; rebase four consolidated tooling registry paths and keep the module's tests/audit runnable | Complete; archived 2026-10-18 |
| 6 | `forge-governance-consumer-migration` | Route Forge workflows to Forge-owned governance operations and remove concrete product references to Workspace Governance paths and provider identity | Complete; archived 2026-10-18 |
| 7 | `forge-quality-checks-portal-ui` | Show project Gate status, check rows, freshness and next action in a dedicated Quality card; keep governance reads under advanced diagnostics; expose Gate profiles in the workbench | Complete; `forge-quality-checks-portal-ui` and `gate-profile-portal-controls` archived 2026-10-18 |
| 8 | `forge-governance-consumer-portability` | Migrate sibling project consumers to versioned Forge assets or commands; verify each project without a governance sibling checkout | Deferred; depends on 6 |
| 9 | `forge-workspace-governance-gate-conformance` | Apply Forge's built-in source-size baseline to the maintained in-tree Python project and split the four over-limit files; prove module Gate scans real candidates | Complete; archived 2026-10-18 |
| 10 | `forge-platform-schema-ownership` | Move only actively consumed schema sources and validators under Forge maintenance; remove sibling-path parity requirements; list unsupported schema families explicitly | Remaining; separate `platform-contracts` ownership |
| 11 | `forge-multiservice-local-runtime` | Represent multiple services and entry points across Compose and local processes; expose the existing `gitrunbykaru` process manager through Forge | Remaining |
| 12 | `forge-platform-libraries-monorepo` | Relocate .NET and Rust libraries as ordinary subdirectories/workspaces while preserving package identity, independent builds and publication | Remaining |
| 13 | `forge-developer-workflow-facade` | Stable end-to-end Forge workflows across consolidated modules; reconcile the current product boundary that excludes generic CI/CD replacement | Remaining |
| 14 | `forge-origin-repository-retirement` | Verify in-tree DriftWatchdog, cargo-* and platform-library sources plus every migrated consumer; preserve local-only history, confirm Workspace Governance origin is already archived, then retire remaining origins | Final gate for remaining origins; depends on source and consumer migrations; requires explicit remote operation |

Before each sibling migration, recheck its live HANDOFF, Git state, build and
publication instructions. Do not overwrite pre-existing sibling work during
source moves. Keep schemas/formats only when an identified producer and
consumer need them, with the owning Forge module and validation included in
the same migration.

## Operating rule

All 29 baseline and audit changes plus the sibling-integration packages
(orders 30 through 39) are archived and promoted to their canonical specs.
When the queue is empty, `HANDOFF.md` carries no `current_spec` pointer — the
governance checker requires the line to be absent, not set to `none`, in that
state. A new implementation cycle selects exactly one eligible change and
follows implement-one-change → local verify/Gate → strict validate → archive,
committing only related work, updating and committing handoff, then stopping
without pushing.

## Idea-to-production workflow changes (delivered)

All four were planning artifacts when this section was written. Each is now
implemented, archived and promoted to its canonical spec, so none of them is a
queue item:

1. `github-cli-project-workflows` — use the developer's existing `gh` session
   for explicit local repository operations; no separate GitHub login.
   → [spec](openspec/specs/github-cli-project-workflows/spec.md)
2. `site-studio-preview-refinement` — create a reviewed site spec, scaffold the
   existing `react-web` profile, run a bounded local preview, and refine via the
   controlled agent adapter.
   → [spec](openspec/specs/site-studio-preview-refinement/spec.md)
3. `project-to-production-workflow` — stage, health-check, and explicitly
   promote a project through OpenPanel; retry Hermora enrollment separately
   without redeploying.
   → [spec](openspec/specs/project-to-production-workflow/spec.md)
4. `hypora-graduation-import` — validate a local `platform.idea-graduation`
   snapshot, preview its brief/evidence, and require confirmation before
   creating Forge project state. Depends on the shared contract and Hypora's
   producer change.
   → [spec](openspec/specs/hypora-graduation-import/spec.md)

Dependencies: Studio consumes existing Intent/generation/agent behavior;
repository operations are independent; production workflow depends on
`github-cli-project-workflows`, OpenPanel's `forge-publish-provider`, and
Hermora's `published-site-onboarding`. The OpenPanel change remains OpenPanel-
owned. Idea graduation follows `platform-contracts` → Hypora export → Forge
import and remains independent of production publishing until user action.
