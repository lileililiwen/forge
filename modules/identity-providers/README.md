# Identity providers

Replaceable identity-provider packs that give a generated project its own
end-user sign-in population. The packs are independent processes. Forge Core
never links a provider library and never speaks a vendor API: it starts a pack
and talks to it over one versioned JSON-lines contract.

This module is in-tree but it is its own Cargo workspace. It is verified on its
own (`forge module verify identity-providers`) and it has no dependency on Forge
Core. A pack that is not installed simply does not exist for Forge.

## The contract

`forge-identity-provider/1.0`. One JSON request object per line on the pack's
standard input, one JSON response object per line on its standard output.

| Field | Meaning |
|---|---|
| `contract` | The contract identifier. A different major version is refused, never adapted. |
| `provider_id` | The pack that owns the request. |
| `operation_id` | The caller's operation identifier. The answer must repeat it. |
| `project_id` | The project the operation belongs to. |
| `project_revision` | The project state the operation is bound to. A changed revision is refused. |
| `operation` | `probe`, `plan`, `apply`, `status` or `stop`. |
| `input` | The bounded, typed operation input. It never carries a secret value. |

A response repeats the identity fields, carries a terminal status
(`ready`, `blocked`, `failed`, `unavailable`), a redacted summary, the plan
changes, the population, secret *references* and recovery actions.

Rules that hold for every pack:

- `plan` is side-effect free. It returns a `plan_digest`.
- `apply` is bound to the confirmed `plan_digest`.
- No answer carries a secret value. A credential is a reference
  (`file://`, `env://`, `config://`, `secret://`).
- Every URL is checked before it is trusted. HTTPS, or plain HTTP on a loopback
  host. A registration link must live on the issuer's own origin.
- A pack that answers for another project, operation or revision is refused.

## The packs

| Pack id | What it does | Modes |
|---|---|---|
| `keycloak-local` | Runs a pinned Keycloak container and gives each project its own realm and its own public OIDC client. | `local-loopback-http` |
| `supabase` | Prepares an isolated Supabase project that the application already owns. It never provisions a project. | `isolated-project` |

The pack descriptors live in `packs/`. Forge reads a descriptor, runs the
executable it names, and never falls back to another provider.

### keycloak-local

- One realm per project: `forge-<project id>`.
- One public OIDC client per realm, with the project's callback path.
- The container image is pinned: `quay.io/keycloak/keycloak:26.0.5`.
- The bootstrap admin password is generated locally and written to
  `<state dir>/<realm>/admin-password` with owner-only permissions. It is
  returned as a `file://` reference, never as a value.
- The admin login is the credential check. When the running runtime does not
  accept the credential the state directory holds, the pack refuses it and
  names the exact reset command. It never removes the runtime or its users on
  its own, because the reset destroys that project's local realm.

The local mode is plain HTTP on loopback. A deployed identity provider must use
HTTPS; the .NET adapter accepts a plain HTTP issuer only for a loopback host in
development.

### supabase

The pack refuses to describe itself as ready until the isolated project has
passed an OIDC and registration capability check (`oidc_capability_verified`).
A Supabase project shared with another product is refused. Provisioning the
project stays an operator task.

## Build and test

```sh
cargo build --release --workspace      # the pack executables
cargo test --workspace                 # the contract fixtures
```

Forge Core finds the pack executables in `target/release/`. Before they are
built, `forge auth setup` reports the pack as `provider_not_built` instead of
failing.

## Limits

- The local Keycloak mode is loopback HTTP, not TLS.
- The Supabase pack is proven against the contract, not against a live Supabase
  project.
- The packs do not render the application's own sign-in and registration
  pages. The application owns those, and the .NET module owns the reusable
  OpenID Connect host adapter.
