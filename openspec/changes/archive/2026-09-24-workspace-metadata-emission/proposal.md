# Proposal: Generate workspace metadata so new projects are adoptable

## Why

Workspace Governance discovers projects through markers — `.project.json` is
the first one — and reports anything without local metadata as
`DISCOVERED_UNREGISTERED`, blocking adoption, gate wiring and Jenkins
manifest eligibility. Every project created by `forge new` carries
`forge.yaml` but no `.project.json`, so Forge's own output is invisible to
the portfolio tooling it is supposed to coordinate (requirement.md §32:
Forge acts as the common project model; it does not replace the
governance plane). Emitting sibling-compatible metadata at generation time
makes each new project adoptable by Workspace Governance on day one while
remaining inert for anything that does not use that system.

## What Changes

- `forge new` additionally writes `.project.json` (workspace-governance
  `schema_version: 1`) with `id`, `kind` (`product` for service/app
  profiles, `control-plane` where the profile declares it), `profile`
  mapped through an explicit Forge-profile→governance-profile table
  (e.g. `rust-web`→`rust-product`, `aspnet-web`→`dotnet-library`-adjacent
  product mapping), `lifecycle: active`, and `verification.command` set to
  the profile's native test command.
- Fields with no honest value stay explicitly absent: `gate_runtime` is
  emitted only when the chosen profile declares one; `evidence_status`
  defaults to `planned`; `deployment.deployable` is `false` until a real
  target exists.
- `--no-workspace-metadata` opts out; `forge import` never creates or
  rewrites an existing `.project.json` (ownership rule applies as with all
  user files).
- Registry records gain no new columns; the metadata file is derived from
  the same manifest data, and generation determinism (hash) covers it.

## BFS Impact Map

- **Capabilities:** `deterministic-project-generation` (output set),
  `project-import` (ownership protection), `profile-registry`/
  `extended-profile-catalog` (profile mapping metadata).
- **Users and flows:** generated projects appear in workspace audits as
  eligible-for-adoption rather than unknown; fleet tooling can read
  verification commands without Forge.
- **Contracts/data/persistence:** consumes the sibling's published
  `.project.json` schema (documented, tolerant extra fields); no Forge
  schema change; generation receipts list the new file.
- **Integrations/configuration:** mapping table lives in profile
  descriptors, so a new profile decides its own governance mapping (or
  opts out); no Workspace Governance code or path assumptions.
- **Callers:** generator, ownership receipts, doctor (presence is
  informational evidence only), upgrade (metadata file is Forge-owned
  unless edited by the user — then ownership conflict applies).
- **Failure/boundary behavior:** no mapping for a profile → file omitted
  with a note, never a guessed governance profile; existing file on import
  → untouched; disabled flag → byte-identical to current output.
- **Tests:** per-profile emission matrix, opt-out parity, import
  preservation, determinism hashes, ownership conflict on modified file.
- **Dependencies:** none of the other proposed changes.
- **Compatibility/security/privacy:** generated projects still build and
  run with zero Forge/governance knowledge (file is inert metadata); no
  host paths, no machine-specific roots stored.

## Capabilities

- `deterministic-project-generation`: generation emits
  workspace-compatible project metadata that makes portfolio adoption
  possible without creating a dependency.

## Non-goals

- Registering projects in Workspace Governance's registry (the sibling's
  own workflow decides adoption).
- Storing verification evidence in `.project.json` beyond the declared
  status vocabulary.
- Retroactively writing metadata into imported projects.
- Requiring the workspace system for any Forge workflow.

Source: requirement.md §14, §15, §32.
