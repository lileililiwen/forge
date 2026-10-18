# Shared contracts and schemas

Contract implementations live in the owning repositories; this reference only
routes there.

## Read first

- `docs/package-backlog.md` — the package index and the coordination rules
  for the contracts packages owned by platform-contracts and the platform
  libraries.
- `schemas/project.schema.json` — the project declaration schema owned here.
- `schemas/registry.schema.json` — the registry schema owned here.
- `schemas/deployment.schema.json` — the deployment declaration schema owned
  here.

## Rules of the road

- platform-contracts owns the language-neutral schema registry; the Rust and
  .NET libraries consume it through their own packages.
- If a package reveals a second lifecycle, owner, or acceptance oracle, stop
  and propose a follow-up package with a dependency edge rather than
  expanding it.
