# Requirements coverage

Source: [Forge Requirements & Product Design v0.3](../requirement.md), preserved in full. Each numbered section maps to a bounded capability below. The 24 baseline capabilities are implemented, verified and archived; mapping indicates delivered behavior with per-cycle evidence in [HANDOFF.md](../HANDOFF.md). Five audit/foundation follow-ups extend the baseline (`runtime-hardening-and-test-isolation`, `profile-and-release-readiness`, `provider-integration-evidence`, `specification-governance-refresh`, `governance-provider-contract-and-local-default`; all archived) and the first sibling-integration package (`external-checker-emission`) is archived as `2026-09-23-external-checker-emission`. Local build/test evidence, fixture-based adapter evidence, native-toolchain matrix evidence and live provider evidence are distinct states: a passing local suite or fixture round trip is never represented as native, provider or release proof. Cross-cutting principles also apply to every package through [architecture](architecture.md) and the local governance rules.

| Section | Requirement area | Planned capability specifications |
| --- | --- | --- |
| 1 | Product definition | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md) |
| 2 | Core problem | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md) |
| 3 | Core philosophy | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md), [profile-registry](../openspec/specs/profile-registry/spec.md), [deterministic-project-generation](../openspec/specs/deterministic-project-generation/spec.md), [semantic-component-registry](../openspec/specs/semantic-component-registry/spec.md) |
| 4 | High-level architecture | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md) |
| 5 | Forge components | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md) |
| 6 | Project manifest | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md) |
| 7 | Project registry | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md) |
| 8 | Project import | [project-import](../openspec/specs/project-import/spec.md) |
| 9 | Profiles | [profile-registry](../openspec/specs/profile-registry/spec.md), [extended-profile-catalog](../openspec/specs/extended-profile-catalog/spec.md) |
| 10 | Feature registry | [feature-lifecycle](../openspec/specs/feature-lifecycle/spec.md) |
| 11 | Fine-grained component registry | [semantic-component-registry](../openspec/specs/semantic-component-registry/spec.md) |
| 12 | Component quality levels | [semantic-component-registry](../openspec/specs/semantic-component-registry/spec.md) |
| 13 | UI standard library | [extended-profile-catalog](../openspec/specs/extended-profile-catalog/spec.md), [semantic-ui-patterns](../openspec/specs/semantic-ui-patterns/spec.md) |
| 14 | Project creation | [deterministic-project-generation](../openspec/specs/deterministic-project-generation/spec.md), [feature-lifecycle](../openspec/specs/feature-lifecycle/spec.md) |
| 15 | Deterministic generation | [deterministic-project-generation](../openspec/specs/deterministic-project-generation/spec.md), [feature-lifecycle](../openspec/specs/feature-lifecycle/spec.md), [validated-intent-planner](../openspec/specs/validated-intent-planner/spec.md) |
| 16 | Natural language Intent | [validated-intent-planner](../openspec/specs/validated-intent-planner/spec.md) |
| 17 | Intent validation | [validated-intent-planner](../openspec/specs/validated-intent-planner/spec.md) |
| 18 | Planner | [validated-intent-planner](../openspec/specs/validated-intent-planner/spec.md) |
| 19 | Skill layer | [ai-procedure-skills](../openspec/specs/ai-procedure-skills/spec.md) |
| 20 | MCP layer | [mature-mcp-surface](../openspec/specs/mature-mcp-surface/spec.md) |
| 21 | AI agent runtime | [agent-runtime-workflows](../openspec/specs/agent-runtime-workflows/spec.md) |
| 22 | Specification system | [specification-remediation](../openspec/specs/specification-remediation/spec.md), [agent-runtime-workflows](../openspec/specs/agent-runtime-workflows/spec.md) |
| 23 | Doctor | [doctor-maturity-assessment](../openspec/specs/doctor-maturity-assessment/spec.md), [external-checker-emission](../openspec/specs/external-checker-emission/spec.md) |
| 24 | DriftWatch integration | [quality-policy-integration](../openspec/specs/quality-policy-integration/spec.md), [external-checker-emission](../openspec/specs/external-checker-emission/spec.md) |
| 25 | Maturity model | [doctor-maturity-assessment](../openspec/specs/doctor-maturity-assessment/spec.md), [central-admin-identity](../openspec/specs/central-admin-identity/spec.md) |
| 26 | Upgrade system | [project-upgrade-orchestration](../openspec/specs/project-upgrade-orchestration/spec.md), [specification-remediation](../openspec/specs/specification-remediation/spec.md) |
| 27 | Distribution | [repository-distribution](../openspec/specs/repository-distribution/spec.md) |
| 28 | Documentation translation | [documentation-translation](../openspec/specs/documentation-translation/spec.md) |
| 29 | Release engine | [release-publishing](../openspec/specs/release-publishing/spec.md) |
| 30 | Deployment | [adapter-deployment](../openspec/specs/adapter-deployment/spec.md) |
| 31 | Central identity and admin SSO | [central-admin-identity](../openspec/specs/central-admin-identity/spec.md) |
| 32 | Existing systems integration | [quality-policy-integration](../openspec/specs/quality-policy-integration/spec.md), [specification-remediation](../openspec/specs/specification-remediation/spec.md), [agent-runtime-workflows](../openspec/specs/agent-runtime-workflows/spec.md), [repository-distribution](../openspec/specs/repository-distribution/spec.md), [ai-procedure-skills](../openspec/specs/ai-procedure-skills/spec.md), [external-planes-analytics](../openspec/specs/external-planes-analytics/spec.md), [external-checker-emission](../openspec/specs/external-checker-emission/spec.md) |
| 33 | Analytics | [external-planes-analytics](../openspec/specs/external-planes-analytics/spec.md) |
| 34 | CLI requirements | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md), [project-import](../openspec/specs/project-import/spec.md), [deterministic-project-generation](../openspec/specs/deterministic-project-generation/spec.md), [doctor-maturity-assessment](../openspec/specs/doctor-maturity-assessment/spec.md), [feature-lifecycle](../openspec/specs/feature-lifecycle/spec.md), [project-upgrade-orchestration](../openspec/specs/project-upgrade-orchestration/spec.md), [specification-remediation](../openspec/specs/specification-remediation/spec.md), [agent-runtime-workflows](../openspec/specs/agent-runtime-workflows/spec.md), [mature-mcp-surface](../openspec/specs/mature-mcp-surface/spec.md), [repository-distribution](../openspec/specs/repository-distribution/spec.md), [documentation-translation](../openspec/specs/documentation-translation/spec.md), [release-publishing](../openspec/specs/release-publishing/spec.md), [adapter-deployment](../openspec/specs/adapter-deployment/spec.md), [external-checker-emission](../openspec/specs/external-checker-emission/spec.md) |
| 35 | API | [core-http-api](../openspec/specs/core-http-api/spec.md) |
| 36 | Portal | [control-plane-portal](../openspec/specs/control-plane-portal/spec.md) |
| 37 | Technology direction | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md), [profile-registry](../openspec/specs/profile-registry/spec.md) |
| 38 | Recommended repository layout | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md) |
| 39 | MVP 0.1 | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md), [profile-registry](../openspec/specs/profile-registry/spec.md), [project-import](../openspec/specs/project-import/spec.md), [deterministic-project-generation](../openspec/specs/deterministic-project-generation/spec.md), [doctor-maturity-assessment](../openspec/specs/doctor-maturity-assessment/spec.md) |
| 40 | MVP 0.2 | [feature-lifecycle](../openspec/specs/feature-lifecycle/spec.md), [project-upgrade-orchestration](../openspec/specs/project-upgrade-orchestration/spec.md) |
| 41 | MVP 0.3 | [quality-policy-integration](../openspec/specs/quality-policy-integration/spec.md), [specification-remediation](../openspec/specs/specification-remediation/spec.md), [agent-runtime-workflows](../openspec/specs/agent-runtime-workflows/spec.md) |
| 42 | MVP 0.4 | [mature-mcp-surface](../openspec/specs/mature-mcp-surface/spec.md) |
| 43 | MVP 0.5 | [repository-distribution](../openspec/specs/repository-distribution/spec.md), [documentation-translation](../openspec/specs/documentation-translation/spec.md), [release-publishing](../openspec/specs/release-publishing/spec.md), [adapter-deployment](../openspec/specs/adapter-deployment/spec.md) |
| 44 | Non-goals | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md) |
| 45 | Long-term product model | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md), [semantic-component-registry](../openspec/specs/semantic-component-registry/spec.md), [validated-intent-planner](../openspec/specs/validated-intent-planner/spec.md) |
| 46 | Ultimate development experience | [semantic-ui-patterns](../openspec/specs/semantic-ui-patterns/spec.md), [validated-intent-planner](../openspec/specs/validated-intent-planner/spec.md) |
| 47 | Central strategic principle | [core-manifest-registry](../openspec/specs/core-manifest-registry/spec.md), [semantic-component-registry](../openspec/specs/semantic-component-registry/spec.md), [semantic-ui-patterns](../openspec/specs/semantic-ui-patterns/spec.md) |

## Interpretation boundaries

- The brief lists six initial profiles in §9 but limits §39 v0.1 to five. `react-web` is planned in v0.2; specialist candidates in §9 remain explicitly unsupported until promoted with evidence.
- §10's feature names are a catalog vocabulary. The feature package requires a tested support matrix and truthful unsupported states, not an unverified promise that all features work on all stacks.
- §13's Blazor UI and §27's GitLab/Codeberg are possible implementations/providers. They remain future adapter candidates.
- §37 is a technology recommendation. Rust/SQLite is the planning baseline, with a foundation ADR required; the portal framework is open.
- §38 is a recommended logical layout. Module ownership is preserved; exact crate/package layout follows the foundation decision.
- `forge.yaml` is the proposed canonical filename. `platform.yaml` is handled only as an explicit legacy import source, with dual-file ambiguity rejected.
- §19 skills cover all named SOPs only after their operations exist. Core retains enforcement; prompts do not implement the platform.
- §46 is a composed long-term acceptance journey across Intent, profiles, features, UI, generation, quality, GitHub and Gitee. It is not part of v0.1.
- Central identity, observability, content and analytics retain their external ownership. Forge coordinates evidence and adapters.
