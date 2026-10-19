# Handoff

current_spec: none

## Current state

The module holds the replaceable identity-provider packs. It is implemented and
verified as part of the root change `forge-auth-provider-extensions`.

## What landed

- `crates/forge-identity-provider` — the `forge-identity-provider/1.0` contract:
  the request and response types, the plan digest, the request and response
  validation, and the secret-free redaction helpers. 13 contract fixture tests
  cover the refusals: unknown contract major, another project, another
  operation, a stale project revision, secret material, a reference that is
  not a reference, a registration URL on another host, a plain HTTP issuer that
  is not loopback, a plan without a digest, and an empty request field.
- `crates/keycloak-pack` — one Keycloak realm and one public OIDC client per
  project on a pinned `quay.io/keycloak/keycloak:26.0.5` container. The
  generated bootstrap admin password is written to a `0600` file and returned
  as a `file://` reference. A runtime that does not accept the stored
  credential is refused with the exact reset command; the pack never destroys
  the runtime or its users on its own.
- `crates/supabase-pack` — an isolated Supabase project only. It refuses a
  shared project, refuses to apply, and refuses to report itself ready before
  the OIDC and registration capability check passes.
- `packs/keycloak-local.json` and `packs/supabase.json` — the descriptors Forge
  Core reads.
- `.project.json`, `.ai-gate/gate.yaml` — the module identity and its own Gate
  declaration, so `forge module list` and `forge module verify
  identity-providers` resolve it.

## Evidence

- `cargo test --workspace` in this module: 13 contract fixture tests pass.
- `forge module verify identity-providers`: the module Gate runs its own
  format, build, test and security checks.
- Real Keycloak evidence was collected on this machine: three realms
  (`forge-alpha-app`, `forge-beta-app`, `forge-demo-shop`), each with its own
  client and its own issuer, an administrator token from one instance refused
  by another, a preview that wrote nothing, a confirmed setup that wrote the
  manifest, and a repeated setup that changed nothing.

## Deferred

- The local mode is loopback HTTP, not TLS.
- The Supabase pack is proven against the contract, not against a live Supabase
  project.
- The module is not registered in the workspace governance registry, so its
  Gate has no `governance-quality` concern.
