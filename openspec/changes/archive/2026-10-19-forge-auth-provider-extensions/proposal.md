## Why

A generated application needs an end-user sign-in population. Today Forge has no
way to give one: the only identity surface is Forge's own administrator login
(`forge identity`) and the per-project administrator OIDC session, and both are
operator surfaces, not product surfaces. An application owner has to stand up an
identity provider by hand, wire it into the generated app, and keep the wiring
correct when the provider changes.

Decision `D9` in the workspace governance register now states the policy: OIDC
is the interoperability protocol, compatible providers are replaceable, no
vendor is required, and a product end-user population is project-scoped by
default. This package is the Forge side of that decision. It gives an owner one
command that prepares the end-user identity of one project, keeps the provider
replaceable, and never binds a provider vendor into Forge Core.

## What Changes

- **A new in-tree module `modules/identity-providers`** with its own Cargo
  workspace. It holds the provider packs as independent processes. Forge Core
  never links a provider library and never calls a vendor API.
- **The `forge-identity-provider/1.0` contract.** One JSON request object per
  line on a pack's standard input, one JSON response object per line on its
  standard output. The contract carries no secret value, every credential is a
  reference, and every URL is checked before it is trusted.
- **`forge auth setup <project>`** in Forge Core. It resolves the project, asks
  the selected pack for a plan, shows the plan, and applies it only when the
  owner confirms. A preview writes nothing.
- **`forge auth provider list` and `forge auth provider inspect`.** The owner
  sees which packs are installed, which mode each one runs in, and whether the
  pack executable is present.
- **Two packs.** `keycloak-local` runs a pinned Keycloak container and gives
  each project its own realm and its own public OIDC client. `supabase`
  prepares an isolated Supabase project that the application already owns, and
  refuses to report itself ready before its OIDC and registration capability is
  verified.
- **A project manifest `auth:` block.** On a confirmed setup the population is
  recorded: provider, population id, issuer, client id, callback path, scopes,
  registration link, and secret *references* only.
- **Typed refusals.** A setup that cannot be done is reported as a preview,
  blocked, declined or refused, with a reason and a recovery instruction. Forge
  Core adds `ForgeError::AuthSetupRefused`.

## Package Boundary and Split Assessment

| Package | Module | Platform/Stack | Status | Decision |
|---|---|---|---|---|
| `forge-auth-provider-extensions` | `forge` | Rust (Forge Core) | implemented | This package. Forge Core contract validation, CLI and manifest write. |
| `provider-neutral-identity-policy` | `forge/modules/workspace-governance` | Policy | implemented | Landed first. Owns the D9 policy. No code. |
| `project-oidc-auth-adoption` | `forge/modules/dotnet-platform-libs` | .NET | implemented | Landed second. Owns the ASP.NET Core OpenID Connect host adapter. |
| `identity-provider-packs` (this module) | `forge/modules/identity-providers` | Rust | implemented | Landed with this package. Owns the pack implementations. |

Three packages, three owners. Forge Core owns the contract and the CLI, the
.NET module owns the ASP.NET Core host adapter, the new module owns the packs,
and the governance module owns the policy. No package edits another package's
files.

## Sibling and Shared Architecture Reconnaissance

| Neighbor | Shared surface | Boundary decision |
|---|---|---|
| `forge/modules/dotnet-platform-libs` | The OIDC vocabulary: issuer, client, callback path, scopes, provider roles. | The .NET module reads the issuer and client that Forge records. It owns the ASP.NET Core scheme, the callback validation and the app-side identity seam. This package does not edit it. |
| `forge/modules/workspace-governance` | The D9 policy: provider neutrality, project-scoped product populations, no shared session or role. | The policy register owns the rule. This package implements it and cites it. It does not edit the register. |
| `forge/modules/rust-platform-libs` | The repository's Rust conventions: workspace layout, toolchain, licence, test style. | The new module follows the same conventions. It shares no crate with the Rust module, because a provider pack must run as an independent process. |
| `forge` publish providers | The idea of a replaceable provider behind a stable contract. | The publish-provider code is not reused. The auth provider is a different contract with a different boundary, and the two must not be coupled. |
| `platform-contracts` | The idea of a versioned serialized contract. | The auth contract is owned by this repository and lives with the packs. It is not added to the shared `platform-contracts` contract set, because no other repository consumes it. |

## User Experience and Interface Impact

The owner-facing surface is the terminal.

```
forge auth setup <project> [--provider <id>] [--confirm]
```

- Without `--confirm` the command prints the plan and writes nothing.
- With `--confirm` it applies the plan and records the population in the
  project manifest.
- `--provider` names a pack. Without it, the first enabled installed pack is
  used.
- `--packs` names the descriptor directory. Without it, the packs that ship in
  the Forge checkout are used.
- `--state-dir`, `--port` and `--callback-path` have working defaults, so the
  common path is one command.
- `--format json` returns the `forge-auth-setup/1.0` envelope with a `status`
  of `preview`, `applied`, `blocked` or `declined`. The pack listing and the
  pack inspection return the `forge-auth-provider-catalog/1.0` envelope, which
  is a catalog and not a setup result.

`forge auth provider list` names every installed pack with its mode and whether
its executable is present. `forge auth provider inspect <id>` returns the
descriptor of one pack.

The application's own sign-in and registration pages are not built by this
package. The application owns them, and the .NET module owns the reusable
host adapter. For the local Keycloak pack the registration link points at the
provider's own hosted registration page, and the pack refuses to report itself
ready when it cannot prove that the provider can register a user.

## BFS Impact Map

- **Forge Core `src/auth/`** — the contract types, the pack catalog, the setup
  runner, the validation and the redaction.
- **Forge Core `src/cli/auth.rs` and `src/cli/commands.rs`** — the `auth`
  command surface.
- **Forge Core `src/core/mod.rs`** — `ForgeError::AuthSetupRefused`.
- **Forge Core `src/lib.rs`, `src/cli/mod.rs`, `src/main.rs`** — module
  registration and dispatch.
- **Forge Core `tests/auth_setup_contract.rs`** — the CLI contract tests.
- **`modules/identity-providers/`** — the new module: the contract crate, the
  two packs, the pack descriptors, the module identity and its Gate.
- **The project manifest** — the `auth:` block, written only on a confirmed
  setup.
- **Unchanged on purpose** — `forge identity`, the publish providers, the
  `platform-contracts` contract set, the DriftWatch engine, the PTY manager,
  and both sibling modules.

## Capabilities

- New capability: `forge-auth-provider-extensions`.
- No existing capability changes. The operator identity surface
  (`forge identity`) and the content and analytics providers are untouched.

## Non-goals

- No change to `forge identity` and no shared session between Forge's operator
  login and an application's end users.
- No provider code in Forge Core, and no reuse of the publish-provider code.
- No change to the .NET module, the workspace governance register, the Rust
  platform module, the `platform-contracts` contract set, DriftWatch, the PTY
  manager, or the content and analytics providers.
- No automatic provider selection for the operator's own identity.
- No TLS for the local Keycloak mode. The local mode is loopback HTTP; a
  deployed provider must use HTTPS.
- No live Supabase verification. The Supabase pack is proven against the
  contract and refuses to report itself ready without a verified OIDC and
  registration capability.
- No application's own sign-in and registration pages. Those belong to the
  application, and the reusable host adapter belongs to the .NET module.
