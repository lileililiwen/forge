## Why

A repository-wide sweep found nine extension surfaces with nothing in common:

| Surface | Descriptor | Version rule | State vocabulary |
|---|---|---|---|
| plugins (`src/plugins/`) | `PluginDescriptor` | none | `PluginState` (4) |
| publish providers (`src/publish/providers/`) | `ProviderEntry` | exact on the wire | none |
| governance providers (`src/governance/`) | `GovernanceProviderConfig` | exact `protocol_version` | `ProviderStatus` (8) |
| identity providers (`src/identity/`, `src/auth/`) | `IdentityConfig` | exact `CONTRACT_ID` | `ProviderStatus` (4) |
| analytics providers (`src/analytics/`) | `AnalyticsProviderConfig` | none | `ProviderSupportStatus` (2) |
| evidence providers (`src/provider/`) | `ProviderDescriptor` | exact | `ProviderStatus` (4) |
| standard packs (`src/standard/`) | `PackDescriptor` | exact id and version | three vocabularies |
| identity packs (`src/auth/packs.rs`) | `PackDescriptor` | none | none |
| components and UI patterns | their own | never compared | two quality vocabularies |

Fourteen distinct state vocabularies. Every version check is exact string
equality, so a newer extension is refused exactly like an unrelated one. The
one place that already had a major number — `CONTRACT_MAJOR` in the
identity-provider module — is never compared.

The deploy target kinds are spread over four places with two different lists.
`src/deploy/contract.rs` declares five kinds; `src/deploy/prepare.rs:28`
accepts five and names four; `src/deploy/model.rs:69` names four; and
`src/deploy/engine/stages.rs:36` refuses `ssh` with a third wording. `ssh` is
advertised, accepted by one validator, refused by another, named by neither.

## What Changes

- **One contract-version rule.** A new `src/extension/` module owns
  `ContractVersion`: one way to read `family/major.minor.patch`, and one rule
  — the families must match and the majors must be equal. The minor and the
  patch are reported, not gating. A value that carries no family cannot be
  read as a contract, so the bare `major.minor.patch` form is refused.
- **One state vocabulary.** The same module owns `ExtensionState`:
  `ready`, `disabled`, `unavailable`, `invalid`, each refusal naming the
  extension and the reason. It is the vocabulary the plugin surface already
  uses, promoted from one surface's private type to the repository's.
- **One kind owner.** The deploy target kinds move into the extension module.
  The manifest validator, the configuration validator and the plan read that
  one list and produce the same wording, and a kind that is known but not
  wired is refused in one place with one message.
- **The deploy executor boundary migrates.** The adapter's declared contract
  is judged by the shared version rule instead of string equality, and the
  adapter's self-identification (`forge-deployer-jenkins/0.1.0`) is read as a
  family and a version. The provider probe, which runs the same adapter and
  reads the same envelope, uses the same rule, so the deploy lane and the
  probe cannot disagree about one adapter.
- **One contract test.** The extension module ships one contract verification
  that a kind runs, so the next kind inherits the rules instead of restating
  them.
- **Nothing else moves.** Every other extension surface, the manifest
  contract, the plugin registry, the release and delivery lanes and every
  sibling module are untouched.

## Package Boundary and Split Assessment

| Package | Module | Platform/Stack | Status | Decision |
|---|---|---|---|---|
| `extension-point-contract` | `forge` | Rust (Forge Core) | implemented | This package. The shared extension contract and the deploy target kind and deploy executor boundary behind it. |
| `extension-contract-convergence` | `forge` | Rust (Forge Core) | planned | Later. The publish provider, the governance provider, the identity provider, the analytics provider, the evidence provider, the packs, the kits, the components and the UI patterns migrate onto the shared contract, one at a time. |
| `plugin-kind-runners` | `forge` | Rust (Forge Core) | planned | Later. Four of the six plugin kinds — `gate`, `quality`, `agent`, `contract` — have a selector and no runner. The contract decides what a runner owes; the runners follow. |
| `deploy-target-ssh` | `forge` | Rust (Forge Core) | planned | Later. The SSH adapter, the first extension that plugs into the port this package opens. |

One package, one owner. The extension contract belongs in Forge Core because
every surface that would use it lives there.

## Sibling and Shared Architecture Reconnaissance

| Neighbor | Shared surface | Boundary decision |
|---|---|---|
| `forge/modules/identity-providers` | The identity provider's `CONTRACT_ID` and its unused `CONTRACT_MAJOR`. | Read only. The identity provider converges in a later change. |
| `forge/modules/dotnet-platform-libs` | Nothing in this change. | Not edited. |
| `forge/modules/rust-platform-libs` | Nothing in this change. | Not edited. |
| `forge/modules/workspace-governance` | The `project.schema.json` interface and capability vocabulary the plugin declaration reads. | Read only. The closed capability set is not widened here. |
| The `platform-contracts` contract set | The contract registry's own major-version allow-list. | Read only. That registry keeps its rule; the shared rule is for Forge's own extension boundaries. |

## User Experience and Interface Impact

Before, an adapter that spoke a newer minor of the same contract was refused
exactly like a stranger:

```
error[deploy-target-unavailable]: deploy target unavailable: deploy adapter contract
`forge-deploy-executor/0.2.0` does not match expected `forge-deploy-executor/0.1.0`
```

After, a newer minor of the same family is accepted, and a major break is
refused with the reason spelled out:

```
error[deploy-target-unavailable]: deploy target unavailable: forge-deployer-jenkins declares
the contract `forge-deploy-executor/1.0.0`, which is incompatible with the required
`forge-deploy-executor/0.1.0`: the major versions differ (1 and 0)
```

And the `ssh` kind is refused in one wording everywhere it appears:

```
error[deploy-invalid]: deploy invalid: deployment target kind `ssh` is known but not
yet wired; v0.1.0 supports `local`, `docker-compose`, `jenkins`, and `mac-runtime`
```

## BFS Impact Map

- **`src/extension/mod.rs`** (new) — the module: `ContractVersion`,
  `ExtensionState`, `ExtensionDescriptor`, the deploy target kind registry,
  and the shared contract verification.
- **`src/extension/version.rs`** (new) — the version reader and the one
  compatibility rule.
- **`src/extension/state.rs`** (new) — the state vocabulary and the
  descriptor.
- **`src/extension/kinds.rs`** (new) — the deploy target kind registry and the
  one refusal wording.
- **`src/extension/contract.rs`** (new) — the shared contract verification.
- **`src/extension/extension_tests.rs`** (new) — the shared contract test
  suite.
- **`src/deploy/contract.rs`** — the target kind constants move to the
  extension module; the executor contract constant stays where the deploy
  surface reads it and is re-exported.
- **`src/deploy/prepare.rs`** and **`src/deploy/model.rs`** — the kind
  validation reads the registry, so the list and the wording exist once.
- **`src/deploy/engine/stages.rs`** and **`src/deploy/engine/model.rs`** — the
  executor's contract check uses the shared version rule, and the `ssh`
  refusal reads the registry.
- **`src/provider/runner.rs`** — the provider probe judges the same deploy
  envelope with the same rule, so the deploy lane and the probe cannot
  disagree about one adapter.
- **`src/provider/matrix.rs`** — the probe's boundary description states the
  rule the probe now applies.
- **`docs/adapter-contracts/deploy-executor.md`** — the contract doc states
  the rule the code now applies.
- **Unchanged on purpose** — the manifest contract, the plugin registry, the
  release and delivery lanes, the publish and governance and identity and
  analytics and evidence providers, the packs, the kits, the components, the
  UI patterns, and every sibling module.

## Capabilities

- New capability: `extension-point-contract`.
- No existing capability changes. The `adapter-deployment` and
  `forge-publish-plugin-orchestration` contracts keep their current
  requirements; this capability adds the rule they were missing.

## Non-goals

- No change to the plugin registry, its descriptor or its discovery order.
- No migration of any other extension surface. The convergence is a later
  change, one surface at a time.
- No runner for the `gate`, `quality`, `agent` or `contract` plugin kinds.
- No new deploy target kind. The SSH adapter is the next change.
- No change to the manifest contract, the schema version or the deploy
  executor's wire format. Only the rule that judges the declared version
  changes.
- No change to the release, delivery, publish or governance lanes.
- No change to any sibling module.
