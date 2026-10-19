## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| Forge Core auth contract, catalog, setup runner | `forge` | Yes |
| Forge Core `auth` CLI | `forge` | Yes |
| Provider pack implementations | `forge/modules/identity-providers` | Yes, as a new module |
| ASP.NET Core OpenID Connect host adapter | `forge/modules/dotnet-platform-libs` | No. Owned by `project-oidc-auth-adoption`. |
| The D9 identity policy | `forge/modules/workspace-governance` | No. Owned by `provider-neutral-identity-policy`. |
| `platform-contracts`, the publish providers, DriftWatch, the PTY manager, the content and analytics providers | their owners | No |

This package adds a new in-tree module. It does not edit the .NET module, the
governance module, the Rust platform module, or any shared contract set.

## Language and Runtime

- **Forge Core** is Rust, in the root `forge` crate. The new code is
  `src/auth/` plus `src/cli/auth.rs`.
- **The packs** are Rust, in `modules/identity-providers`, which is its own
  Cargo workspace with three crates: `forge-identity-provider` (the contract),
  `keycloak-pack` and `supabase-pack`.
- **The local Keycloak runtime** is a container on the pinned image
  `quay.io/keycloak/keycloak:26.0.5`, reached over plain HTTP on loopback.
- The module is verified on its own through `forge module verify
  identity-providers`, which runs its own format, build, test and security
  checks. It is not part of the root `cargo test --workspace`, because it is a
  separate workspace.

## Ownership and Shared Code

A provider pack is an independent process. It runs on its own, it owns its own
credentials, and it can be replaced without touching Forge Core or an
application.

That boundary is why the contract types exist twice:

- `modules/identity-providers/crates/forge-identity-provider` owns the types a
  pack uses.
- `forge/src/auth/contract.rs` owns the types Forge Core uses.

They are not a shared crate. A shared crate would be a compile-time link
between the two sides, and the whole point of the boundary is that the two
sides can change independently. The wire format is the contract, and the
contract is versioned. The two copies are kept in step by the contract
fixtures in the module and by the CLI contract tests in Forge Core.

Forge Core never calls a vendor API, never embeds a provider SDK, and never
falls back to a different provider when the selected one is missing. The
publish-provider code is not reused: it is a different contract for a different
boundary.

## User Experience and Interface

```
forge auth setup <project> [--provider <id>] [--packs <dir>] [--state-dir <dir>]
                          [--port <n>] [--callback-path <path>] [--confirm]
forge auth provider list [--packs <dir>]
forge auth provider inspect <id> [--packs <dir>]
```

- The default pack is the first enabled installed pack.
- The default pack directory is the `packs/` directory in the Forge checkout,
  or `$FORGE_ROOT/packs`.
- The default state directory is `<project>/.forge/identity`.
- The default callback path is `/signin-oidc`; the default port is `18080`.
- `--format json` returns the `forge-auth-setup/1.0` envelope.

The human output of a preview names the provider, the planned changes and the
plan digest, and ends with `Confirm with --confirm to apply.` A blocked
outcome names the reason and the recovery instruction. A confirmed setup names
the population, the issuer, the client and the manifest path.

## Behavioral Model

| Actor | Population | Boundary |
|---|---|---|
| The Forge operator | Forge's own administrator identity (`forge identity`) | Untouched by this package. |
| The application end user | One population per project | The local Keycloak pack gives each project its own realm and its own client. The Supabase pack refuses a project shared with another product. |

An application end user never signs in through Forge's operator identity, and
two projects never share an account, a session, a role or a permission. The
provider's role claims grant no local permission: the application grants access
explicitly. That is the D9 rule, implemented as a refusal inside the pack
contract rather than as a convention.

The setup is a three-step exchange:

1. **probe** — the pack reports whether it can serve this project.
2. **plan** — the pack returns the changes it would make and a plan digest.
   The plan is side-effect free.
3. **apply** — only after confirmation, bound to the confirmed plan digest.

## Contract and Compatibility

`forge-identity-provider/1.0`. One JSON request object per line on the pack's
standard input, one JSON response object per line on its standard output.

Request fields:

| Field | Rule |
|---|---|
| `contract` | Must be `forge-identity-provider/1.0`. Another major version is refused, never adapted. |
| `provider_id` | The pack that owns the request. The answer must repeat it. |
| `operation_id` | The caller's operation identifier. The answer must repeat it. |
| `project_id` | The project the operation belongs to. The answer must repeat it. |
| `project_revision` | A digest of the project manifest. The answer must repeat it. |
| `operation` | `probe`, `plan`, `apply`, `status` or `stop`. |
| `input` | The bounded operation input. It never carries a secret value. |

Response fields: the same identity fields, plus `status` (`ready`, `blocked`,
`failed`, `unavailable`), `summary`, `proposed_changes`, `population`,
`secret_references`, `recovery` and `plan_digest`.

Rules that Core enforces on every answer:

- The contract major must match.
- The answer must be for the same provider, operation, project and revision.
- The answer must not carry secret material. The scan runs on the raw answer,
  before the typed response drops the fields the contract does not define, so a
  secret sent in an unknown field is still refused. A value that is a
  reference (`file://`, `env://`, `config://`, `secret://`) is allowed; a value
  that is a secret is not.
- Every URL must be HTTPS, or plain HTTP on a loopback host. A registration
  link must live on the issuer's own origin.
- A plan must carry a plan digest. The digest is the SHA-256 of the canonical
  plan bytes, and `apply` must repeat the digest that was confirmed.

Compatibility: the contract is versioned and a different major version is
refused. The `auth:` block in the project manifest is additive. A project
without an `auth:` block is unaffected, and a project whose manifest cannot be
read is refused before the provider is called.

## Failure and Boundary Policy

| Situation | Outcome |
|---|---|
| The pack is not installed | Blocked, `provider_unknown`. Forge does not fall back to another provider. |
| The pack is disabled | Blocked, `provider_disabled`. |
| The pack executable is not built | Blocked, `provider_not_built`. |
| The project manifest already names another provider | Blocked, `existing_auth_conflict`. |
| The pack is not ready, blocked, failed or unavailable | Blocked with the pack's own status and its recovery actions. |
| The pack answers for another project, operation or revision | Refused. |
| The answer carries secret material | Refused. |
| The answer carries an unsafe URL | Refused. |
| The plan carries no digest | Refused. |
| The pack reports success without a population | Blocked, `provider_returned_no_population`. |
| The owner declines the confirmation | Declined. Nothing is written. |
| The pack fails after it changed external state | Blocked with the recovery action. The project manifest is not written, and the provider's own data is not deleted. |
| The local runtime does not accept the stored credential | Blocked, `provider_credential_mismatch`, with the exact reset command. The runtime and its users are left alone. |

A blocked or refused setup never writes the project manifest, and a preview
never writes anything at all.

## Verification Oracle

```sh
# Forge Core contract tests
cargo test --test auth_setup_contract

# The module's own contract fixtures
cargo test --workspace --manifest-path modules/identity-providers/Cargo.toml

# The module's own Gate
forge module verify identity-providers

# The repository Gate
forge gate --dry-run
forge gate --timeout-secs 1800

# Strict OpenSpec validation and whitespace
openspec validate --all --strict --no-interactive
git diff --check
```

The oracle is the behaviour, not the build. A passing build is not evidence:
the evidence is that a preview writes nothing, that a confirmed setup records
exactly one population, that a second setup of the same provider changes
nothing, that two projects get two populations, that a secret in the answer is
refused, and that a real Keycloak instance gives two projects two issuers.

## Implementation Plan

**The module.** `modules/identity-providers` is a Cargo workspace with
`forge-identity-provider`, `keycloak-pack` and `supabase-pack`. It has its own
`.project.json` and `.ai-gate/gate.yaml`, so `forge module list` resolves it and
`forge module verify identity-providers` runs its own checks. The module's
`README.md` states the contract, the packs and the limits.

**The contract crate.** `forge-identity-provider` holds the request and
response types, the plan digest, the request and response validation and the
redaction helpers. Thirteen fixture tests cover the refusals: an unknown
contract major, another project, another operation, a stale revision, secret
material, a value that pretends to be a reference, a registration URL on
another host, a plain HTTP issuer that is not loopback, a plan without a digest,
an empty request field, a secret in the summary, a secret in the plan changes,
and a well-formed answer that is accepted.

**The Keycloak pack.** It derives the realm name from the project id
(`forge-<project id>`), runs a container on the pinned image, and gives the
realm one public OIDC client with the project's callback path. It generates a
bootstrap admin password, writes it to `<state dir>/<realm>/admin-password`
with owner-only permissions, and returns it as a `file://` reference. The admin
login is the credential check: when the running runtime does not accept the
stored credential, the pack refuses it, names the exact reset command, and
leaves the runtime and its users alone. The pack never resets the runtime on
its own, because the reset removes that project's local realm.

**The Supabase pack.** It refuses a project that is shared with another
product, refuses to apply, and refuses to report itself ready before the
`oidc_capability_verified` check passes. Provisioning stays an operator task.

**Forge Core.** `src/auth/contract.rs` holds Core's copy of the wire types, the
validation and the redaction. `src/auth/packs.rs` reads the descriptors and
reports which packs are installed, enabled and built. `src/auth/setup.rs` runs
the probe, the plan and the apply, and writes the manifest. `src/cli/auth.rs`
is the terminal surface.

**The manifest write.** The `auth:` block is written into the project manifest
as YAML. It holds the schema version, the provider, the population id, the
issuer, the client id, the callback path, the scopes, the registration link and
the secret references. It never holds a secret value. A project that already
names the same provider is not written twice.

**The CLI.** `forge auth setup` and `forge auth provider list|inspect` are
registered in `src/cli/commands.rs` and dispatched from `src/main.rs`. The
`ForgeError::AuthSetupRefused` variant carries a refused setup.

## Decision Ledger

- **Resolved:** a provider pack is an independent process behind a versioned
  JSON-lines contract. Forge Core links no provider code.
- **Resolved:** Core keeps its own copy of the wire types. The process boundary
  is worth more than the shared crate.
- **Resolved:** the plan is previewed and confirmed. The digest binds the apply
  to the plan that was shown.
- **Resolved:** the manifest records references, never values.
- **Resolved:** the local Keycloak mode is loopback HTTP. A deployed provider
  must use HTTPS, and the .NET adapter accepts plain HTTP only for a loopback
  host in development.
- **Resolved:** the Supabase pack reports itself blocked until its OIDC and
  registration capability is verified.
- **Deferred:** the generated application's own sign-in and registration pages
  and their browser and accessibility evidence. They belong to the application
  template, not to this package.
- **Deferred:** a live Supabase verification. The pack is proven against the
  contract only.
- **Implementation authorized (2026-10-19):** the owner directed the
  implementation of this package after the D9 policy and the .NET adapter had
  landed. The authorized surfaces are the files listed in the proposal.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| Application auth setup is explicit and separate from the operator identity | Add a project-scoped `forge auth setup` | The owner runs one command and reads the plan | The auth surface is separate from `forge identity` | `src/auth/`, `src/cli/auth.rs` | The operator surface is untouched; a project setup is recorded only on confirmation | 2.1–2.3 | CLI contract tests |
| Auth setup uses preview and explicit confirmation | Two steps: preview, then apply | `Confirm with --confirm to apply.` | The apply is bound to the confirmed plan digest | `src/auth/setup.rs` | A preview writes nothing; a decline writes nothing; a confirmed setup writes once | 2.4, 3.1 | CLI contract tests |
| Provider packs are replaceable and independently bounded | A new module with a versioned contract | `forge auth provider list` names the packs | An independent process, two copies of the wire types | `modules/identity-providers/` | A missing pack is blocked, never replaced by another | 2.5–2.7 | Contract fixtures, module Gate |
| Provider plan and application change are safe | The plan is side-effect free | The owner reads the planned changes | The digest binds the plan to the apply | `src/auth/setup.rs` | A failed apply blocks and writes nothing | 2.8, 3.2 | CLI contract tests, real Keycloak run |
| Product end-user populations remain project-isolated | One realm per project | The owner sees the population id | A project id names the realm and the client | `crates/keycloak-pack`, `crates/supabase-pack` | Two projects get two realms, two clients and two issuers | 2.9–2.10 | Real Keycloak evidence |
| The recorded provider configuration is standards-based and fails closed | OIDC, checked URLs, no secret values | The owner sees the issuer and the registration link | HTTPS or loopback, same-origin registration, references only | `src/auth/contract.rs`, both packs | A secret, an unsafe URL or an off-origin link is refused | 2.11–2.12 | Contract fixtures, CLI contract tests |
| The setup reports a usable outcome | A typed outcome and a manifest entry | The human output and the JSON envelope | `forge-auth-setup/1.0` | `src/cli/auth.rs` | A blocked setup names the reason and the recovery | 2.13–2.14 | CLI contract tests |
