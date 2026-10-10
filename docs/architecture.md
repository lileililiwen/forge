# Architecture and decision boundaries

## Product constraints

Forge starts as a modular monolith. Core owns project identity, manifests, capability contracts, validation and operation outcomes. The product model is independent of stack, AI provider, IDE, cloud and hosting. Generated projects retain ordinary owned source, native build/test commands and no required Forge runtime.

Deterministic assets are preferred at every reuse level: Profile → Feature → Pattern → Capability. Natural language becomes validated Intent, then a reviewable capability graph and pinned plan. Only unresolved glue, business logic and semantic migrations go to AI/spec work. Natural language never becomes raw shell commands.

## Module ownership

| Boundary | Ownership | Consumers |
| --- | --- | --- |
| Core / Registry | Validated project model, schema, identity, state and timestamped observations | All operations |
| Profiles / Features / Components | Versioned assets, contracts, compatibility, provenance and quality evidence | Planner, Generator, Upgrade, Doctor |
| Intent / Planner | Structured intent, graph validation, exact resolution and reviewable steps | CLI, later MCP/API |
| Generator / Upgrade | Deterministic application, ownership records, preconditions and recovery | New, feature lifecycle, upgrades |
| Policies / Doctor | Maturity assessment, local inspection and normalized external policy evidence | CLI, release, later transports |
| Agent / Integrations | Existing PTY manager, DriftWatch, content and analytics adapters | Spec execution, quality and observations |
| Gate Runtime Consumption | Driftwatchdog owns plan resolution, execution, blocking policy, its own run history and exit semantics; Forge owns bounded invocation, revision-bound evidence, journaling and honest surface projection; the sibling's `gate evidence-export` verb produces a versioned document that Forge consumes (vocabulary gate, revision gate, attribution, contradiction rule, atomic persistence, `forge gate evidence` read surface) | CLI (`forge gate`, `forge gate evidence`), Doctor, Release |
| Distribution / Release / Deployment | Explicit external operations and durable per-stage outcomes | CLI, later mature MCP/API |
| CLI / MCP / API / Portal | Input/output transport only; no duplicated business rules | Humans, agents, portal views |

The [requirement.md](../requirement.md) §38 tree is logical guidance; the Core modules above implement it.

## Technology direction

The brief suggests Rust Core/CLI, SQLite initially, YAML manifests and stdio MCP. These are the implemented baseline, recorded with exact toolchain, crate boundaries, schema/parser and SQLite strategy in the foundation ADR ([ADR 0001](adr/0001-foundation-toolchain.md)). PostgreSQL is conditional on demonstrated need. The portal ships as a read-only Core/CLI data surface that points at the SPA as the single interactive browser surface ([portal–SPA](portal-spa.md)); an ASP.NET Core or Next.js graphical portal remains a deferred downstream choice.

Adopt `forge.yaml` as canonical. An explicit legacy import may read `platform.yaml`, but ambiguous coexistence blocks mutation. Schema versions must be supported explicitly. Manifests describe desired infrastructure; observations include source revision, time and actual verification state. Credentials are references to external secret storage, not embedded tokens.

## Mutation and failure model

Validate inputs and compatibility before side effects. Use typed arguments, constrained paths, pinned assets, explicit destinations and project-scoped authority. Preserve user-owned files and unrelated Git changes. Plan destructive or remote operations with exact targets and recovery implications.

Stage file generation and validate before promotion. Reconcile manifest and registry partial writes through operation records. Record per-project and per-provider outcomes for upgrades, releases and deployment. External publication and data migrations are not generally reversible; never advertise universal rollback.

Retries verify input hashes, source revisions and completed stages. Stale plans require review again. Missing tools, unsupported capabilities, inaccessible paths, unavailable integrations and stale observations must not become PASS, healthy or complete.

## Integration boundaries

DriftWatch owns the quality/policy engine; its actual interface is inspected when integrating. The existing PTY manager owns process/session mechanics for OpenCode, Codex and future agents. Existing GitHub analytics and unified content systems retain their identities. The existing Codex/OpenSpec proposal workflow owns bounded semantic-change authoring.

Transport parity is delivered: CLI first, one mature stdio MCP server, an authenticated HTTP API and read-only portal views afterward, all dispatching through the same Core contracts. SSO uses standard OIDC, project-specific sessions and independent authorization; no shared cookies across unrelated apps.

## Evidence

Test deterministic output and repeatability, compatibility rejection, path confinement, partial writes, stale plans and per-project/provider failures. Run generated-project native builds for each advertised profile. Integration fixtures prove contracts; successful real adapter runs establish runtime integration. Neither OpenSpec validation nor build-only skeletons establish product completion.
