# Forge roadmap

Status: 24 baseline entries are implemented and archived; four audit follow-up changes are queued in `openspec/changes/`. The follow-ups address runtime/test hardening, native/release evidence, real provider evidence and specification governance. Versions are delivery targets, not release promises. [requirement.md](requirement.md) is authoritative; [coverage](docs/requirements-coverage.md) accounts for every numbered section.

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

## Release acceptance

- **v0.1:** all five MVP profile fixtures generate and build using native toolchains without Forge; import preserves existing source; registry persists; doctor distinguishes missing, inapplicable and unverified evidence. Missing toolchains block a claim of verified profile support.
- **v0.2:** tested feature/profile support matrix, safe add/remove/upgrade, repeatable migrations and isolated fleet failures. Unsupported catalog entries stay explicit.
- **v0.3:** real DriftWatch and agent-manager contract evidence, traceable spec remediation, tests and scoped Git operations. Fakes alone are insufficient integration evidence.
- **v0.4:** transport parity and negative authorization/input tests for every exposed mature tool.
- **v0.5:** provider sandbox or controlled integration evidence for mirrors, translations, release stages and deployment health; partial external outcomes remain partial.
- **Later:** each bounded package was verified independently; no portal/planner/SSO claim follows from the earlier CLI milestones alone — each has its own archived evidence.

## Deferred choices

The brief recommends Rust Core/CLI, SQLite initially, YAML and stdio MCP. Versions and persistence approach are recorded in the foundation ADR; product contracts are not bound to Rust. HTTP authentication and remote exposure were decided in their own changes. The portal ships as a read-only Core/CLI data surface (`forge portal dashboard|view`); a graphical portal framework (ASP.NET Core / Next.js) remains a deferred downstream choice. PostgreSQL registry migration, specialist profiles, Blazor UI adapters and additional Git providers remain conditional future candidates. No unsupported runtime command is represented as implemented.

## Operating rule

The 24 baseline changes are archived; the audit queue is intentionally active. A new implementation cycle starts with exactly one eligible change, setting the single `current_spec` pointer, and following implement-one-change → local verify/Gate → strict validate → archive, committing only related work, updating and committing handoff, then stopping without pushing.
