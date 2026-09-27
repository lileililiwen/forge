# Proposal: forge-publish-plugin-orchestration

## Why

Forge should be the only user-facing publish system. GitHub pushes and manual
commands must enter the same publish engine, while OpenPanel and Jenkins remain
optional providers that can be enabled or disabled independently. OpenPanel is
already a broad server-management panel with reusable application-delivery and
deployment-adapter contracts; it must not be reduced to a deployment script.

## What Changes

- Add a Forge-managed provider-plugin registry with explicit enable/disable
  lifecycle and capability discovery.
- Define one provider-neutral publish contract for push-triggered and manual
  publishing.
- Add CLI forms `forge publish --project <id>` and
  `forge publish --folder <path>`.
- Accept a verified GitHub push event and route it to the same publish engine
  with its exact commit SHA.
- Adapt OpenPanel to the contract through its existing application-delivery
  and deployment-adapter layers.
- Keep Jenkins as an optional compatibility plugin, not a second control
  plane.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-publish-plugin-orchestration` | One Forge publish engine and switchable provider contract | Forge / Rust | `forge-publish-provider/0.1.0`, CLI, push event | Existing Forge release/deploy contracts | Forge unit and CLI contract tests |
| `forge-openpanel-publish-provider` | OpenPanel implements the Forge provider contract | OpenPanel / Rust | JSON-RPC provider process | Forge provider contract | OpenPanel adapter conformance tests |
| `forge-jenkins-compat-provider` | Existing Jenkins behavior becomes an optional provider | jenkins-local / Python or Rust adapter | Same JSON-RPC provider contract | Forge provider contract | Provider enable/disable and compatibility tests |

The first package defines the stable dependency. The two provider packages are
independently deployable consumers and must not redefine the contract. Neither
provider is moved into Forge or made a Forge workspace dependency.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| OpenPanel | `crates/openpanel-domain/src/deployment_adapters/logic.rs`, `application_delivery`, `git_deployment` | Provider-neutral actions, idempotency, rollback, webhook verification, build/runtime adapters | In-process Rust traits versus Forge external provider contract | OpenPanel owns its panel; Forge owns orchestration | **Adapt through a generic adapter** |
| OpenPanel plugin system | `crates/openpanel-domain/src/plugin/manifest.rs` | JSON-RPC/Wasm plugin runtime and capability declarations | Existing lifecycle is panel-plugin lifecycle, not Forge publish lifecycle | OpenPanel owns plugin loading | **Adapt through a generic adapter** |
| Forge deploy/release | `src/deploy`, `src/release`, adapter contracts | Existing confirmation, evidence, artifact, and revision semantics | No provider lifecycle or push-triggered publish | Forge | **Extend shared owner** |
| Jenkins-local | `deploy-all.sh`, `project-action.sh`, legacy Jenkins jobs | Existing operational behavior and recovery knowledge | Mac-local scripts violate the new ownership boundary | jenkins-local compatibility provider | **Adapt through a generic adapter** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Forge CLI | Add manual project/folder publish forms and plugin lifecycle commands |
| Forge event intake | Verify GitHub push identity, repository, ref, and commit SHA |
| Forge persistence | Store provider enablement, publish run identity, and terminal evidence |
| Forge provider contract | Add capability, preflight, publish, verify, and rollback messages |
| OpenPanel | Add an external Forge provider facade over existing services; remain standalone |
| Jenkins-local | Add a compatibility provider; remain standalone; no Mac script installation |
| Security | Provider secrets stay provider-local; disabled providers are never invoked |

## Capabilities

1. `forge-publish-orchestration` — one publish engine for manual and push
   triggers.
2. `forge-provider-lifecycle` — list, enable, disable, and inspect providers.
3. `forge-publish-provider-contract` — stable external provider protocol.

## Non-goals

- Rewriting OpenPanel’s application-delivery or deployment-adapter domains.
- Making Jenkins required for Forge publishing.
- Maintaining deployment scripts or source checkouts as Mac authority.
- Building through GitHub Actions.
- Implementing a marketplace for arbitrary untrusted Forge plugins.
