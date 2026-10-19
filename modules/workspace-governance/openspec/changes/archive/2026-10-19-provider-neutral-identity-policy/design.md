## Implementation Boundary

Policy owner: the in-tree `forge/modules/workspace-governance` project. Add D9 to its `decisions.json`, mark D2 `superseded` without rewriting its history, and update the owning `workspace-decision-register` capability. Update only governance-owned policy documentation when implementation is later authorized. Do not edit Forge Core, .NET, Rust, platform-contracts, or jenkins-local files in this package.

## Language and Runtime

Policy data is JSON validated by the module's Python workspace audit. This change has no runtime service, generated application, or deployment target. Implementation verification belongs to `forge/modules/workspace-governance`'s Python tests and Gate.

## Ownership and Shared Code

The in-tree decision register owns cross-project policy. Forge Core owns product orchestration and provider packs; the in-tree .NET platform module owns ASP.NET Core authentication adapters; Rust libraries own Rust reusable identity contracts; `platform-contracts` owns only shared serialized payload contracts. No shared code is added here. Consumers reference this policy by decision ID and versioned OpenSpec package; they do not import governance code at runtime.

## User Experience and Interface

N/A for this policy-only package: no human-facing workflow or UI is introduced. The operator-facing interface remains the existing decision register and documentation. The dependent Forge and .NET packages own the actual app setup and login/registration journeys.

## Behavioral Model

| Identity population | Sharing | Default authorization |
|---|---|---|
| Workspace administrators | May authenticate through one operator-selected workspace OIDC provider | Provider authentication does not grant product permissions; each application grants admin explicitly |
| Product end users | Distinct population per product/project | Authentication is project-scoped; no account, session, role, or permission is shared implicitly |
| Public static-site visitors | No shared login | Public access remains as currently published |

A provider may be changed only by a separately authorized operator/runtime action. Replacing the policy does not mutate existing provider deployments or user records.

## Contract and Compatibility

Add D9 as the next permanent identity decision, superseding D2 without rewriting D2's historical entry. Retain D2's internal-workspace boundary and public-static-site exclusion in D9. State that OIDC is the provider interoperability protocol and compatible vendors are replaceable. Keep the existing Zitadel deployment as current operational configuration until a separately authorized migration. No schema migration or API change.

## Failure and Boundary Policy

- Missing or incompatible OIDC provider: login unavailable/denied; no fallback identity and no cross-project access.
- Valid authentication without application authorization: deny access.
- Product provider population configured for more than one product: reject the default setup plan or require an explicit, separately reviewed exception; authentication must not imply shared authorization.
- Public static site: never attach workspace SSO based on this policy.
- Existing runtime not migrated: remains operational under its present configuration; this package makes no status claim about deployment health.

## Verification Oracle

Implementation is authorized, so the oracle runs now: run the in-tree governance module's strict OpenSpec validation and policy audit. Assert D9 is provider-neutral, internal-only, preserves public-site exclusion, identifies shared-admin versus isolated-product populations, supersedes D2 without rewriting it, and does not declare a runtime migration. Review all consumers for agreement. Strict validation is not implementation evidence.

## Implementation Plan

**Register mutation.** `decisions.json` keeps `version: 1` and the existing
entry order. The D2 entry changes one field only: `status` becomes
`superseded`. Its `id`, `title`, `date`, `owner_project`, `decision` text,
`policy_boundary` text and `downstream` list are byte-identical, so the
historical record stays intact. A new entry is appended after D8:

| Field | Value |
|---|---|
| `id` | `D9` |
| `title` | `Provider-neutral workspace identity policy` |
| `status` | `accepted` |
| `date` | `2026-10-19` |
| `owner_project` | `workspace-governance` |
| `decision` | Standard OIDC is the interoperability protocol for workspace identity. Compatible identity providers are replaceable, and no vendor is required by policy. Workspace administrators may authenticate through one operator-selected workspace OIDC provider, but that authentication grants no application permission by itself. Product end-user populations are project-scoped by default: one distinct population per product, with no account, session, role or permission shared implicitly across projects. The current Zitadel deployment remains the operator's present configuration until a separately authorized runtime change; this decision declares no migration and makes no claim about deployment health. |
| `policy_boundary` | Shared workspace SSO is internal-only and stays limited to internal workspace products. The public GitHub Pages catalog (`lileililiwen-site` and any sibling published as a static site) remains excluded from shared login and must not consume workspace SSO, shared analytics, or any cross-product identity data. Provider authentication alone never grants product or administrator authorization: each application grants access explicitly. Configuring one product end-user population for more than one product is refused by default. |
| `downstream` | `dotnet-platform-libs: project-oidc-auth-adoption`, `forge: forge-auth-provider-extensions` |

The downstream change names are the OpenSpec packages that consume this
policy. Both are kebab-case and both repository ids are registered in
`projects.json`, so the loader's registry check passes.

**Loader and audit.** `scripts/decisions.py` needs no change: `superseded`
is already a legal status and unknown entry fields are tolerated. The
`REG-004` summary in `workspace_check.py --decisions` prints the stored
status, so D2 renders as `superseded` without a code change.

**Human view.** `docs/toolchain-decisions.md` lists D2 as `superseded`, adds
the D9 row and a `## D9` section that restates the same policy in prose, and
adds a register rule that names the D9 boundary. The existing D2 section is
kept and marked superseded, so the prose view still shows what D2 decided.

**Tests.** `tests/test_decisions.py` currently asserts that every register
entry is `accepted`; the lifecycle rule in REG-003 allows `superseded`, so
that assertion is narrowed to the accepted-entry rule and a separate test
covers the supersede transition. New tests assert that D9 exists, that it
names standard OIDC and replaceable providers, that it names no mandatory
vendor, that it separates workspace administrators from product end users,
that it keeps the public static-site exclusion, that D2 kept its original
text, and that D2's downstream reference to `identity-oidc-client` is still
present as history.

**Verification commands.**

```sh
python3 -m unittest discover -s tests -v
python3 scripts/workspace_check.py --self-check
python3 scripts/workspace_check.py --decisions
openspec validate --all --strict --no-interactive
git diff --check
```

## Decision Ledger

- **Resolved:** add next permanent decision D9, superseding D2 without rewriting its historical entry; standard OIDC and replaceable providers; current Zitadel deployment remains until a separate runtime change.
- **Resolved:** administrator SSO can be shared; product user populations are isolated by default.
- **Deferred:** any actual shared-provider migration and its runbook, requiring a separate deployment decision.
- **Implementation authorized (2026-10-19):** the owner directed implementation of this package; the register, its human view, the canonical spec and the module tests are the only authorized surfaces.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow / N/A | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| Identity-provider policy preserves vendor choice and population boundaries (REG-005) | Replace mandatory single vendor with OIDC policy | N/A: policy-only; downstream Forge/.NET packages own user journeys | D9 supersedes D2; no runtime migration | `decisions.json`; `workspace-decision-register` | Supersede, shared admin vs isolated users, public static exclusion | 2.1–2.3, 3.1–3.2 | Register lifecycle and policy audit |
