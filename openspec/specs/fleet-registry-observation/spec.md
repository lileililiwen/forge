# fleet-registry-observation Specification

## Purpose

Project an external workspace portfolio registry into read-only Forge fleet
observations so fleet surfaces reflect the real portfolio without Forge
adopting or mutating it.
## Requirements
### Requirement: Read-only fleet projection

Forge SHALL parse a declared workspace registry document under its schema
version contract and SHALL report its entries as timestamped fleet
observations without writing to the registry file, the referenced project
directories or the local project registry.

#### Scenario: Portfolio listed

- **WHEN** an operator runs `forge fleet list` against a valid registry
- **THEN** each entry shows id, declared profile, lifecycle, adoption,
  whether a `forge.yaml` exists and whether the project is locally managed,
  with the registry path and observation timestamp on the report

#### Scenario: Registry untouched

- **WHEN** any number of fleet calls run against a fixture tree
- **THEN** every file hash in the tree is unchanged

### Requirement: Path confinement and per-entry isolation

Forge SHALL refuse entries whose canonicalized path escapes the registry's
workspace root or traverses a symlinked escape, naming the entry, and SHALL
report remaining valid entries despite isolated malformed ones.

#### Scenario: Escaping entry

- **WHEN** an entry path resolves to `../outside-root`
- **THEN** that entry is reported malformed and excluded, and the rest of
  the fleet still renders

#### Scenario: Duplicate identity

- **WHEN** two entries share one id
- **THEN** the report refuses with a typed fleet-registry-invalid naming the
  duplicate rather than choosing one silently

### Requirement: Honest freshness and unmanaged state

Forge SHALL classify registry freshness from the file mtime against the
configured maximum age, SHALL label entries without a local registry record
as `unmanaged`, and SHALL never let a stale or unconfigured fleet source
render as healthy on any read surface.

#### Scenario: Stale registry

- **WHEN** the registry file is older than the max age
- **THEN** reports and portal fleet blocks carry `stale` and the portal
  roll-up is at most `warn`

#### Scenario: Unconfigured

- **WHEN** no registry path is configured
- **THEN** fleet surfaces report `unconfigured` and every local Forge
  workflow continues unaffected

