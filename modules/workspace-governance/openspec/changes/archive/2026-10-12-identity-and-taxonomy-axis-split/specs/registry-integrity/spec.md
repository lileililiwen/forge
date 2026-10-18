# registry-integrity Specification (delta: identity-and-taxonomy-axis-split)

## MODIFIED Requirements

### Requirement: Canonical id is derived at authoring time

`init_project.py` MUST derive the project `id` from the target directory
name using the kebab rule, MUST refuse an explicit `--id` that differs from
the derived id, and MUST allow an explicitly recorded alias only when the
directory name is not a valid identifier. The derived `id` MUST thereafter be
the project's immutable identity: it MUST NOT be renamed to match the
directory, and the registry `path` MUST NOT be rewritten to match the `id`.
An entry whose `id` differs from its `path` in spelling, and whose `path`
resolves and whose declaration `id` equals the registry `id`, MUST NOT be
reported as a ghost, an identity mismatch, or a taxonomy finding.

#### Scenario: Derived id

- **WHEN** `init_project.py` targets a directory named `hestiaLab`
- **THEN** the written declaration's `id` is `hestia-lab`

#### Scenario: Conflicting explicit id refused

- **WHEN** `init_project.py` is given an `--id` that differs from the derived
  id for a non-dotted directory
- **THEN** it refuses and writes nothing

#### Scenario: Dotted directory alias

- **WHEN** the directory name is not a valid identifier, such as
  `lileililiwen.github.io`
- **THEN** an explicit alias is accepted and recorded with the real `path`

#### Scenario: An existing alias entry is not a finding

- **WHEN** a registry entry carries `id: campus-mesh`, `path: campusMesh`, a
  resolvable path, and a declaration whose `id` is `campus-mesh`
- **THEN** the audit reports no identity, path, or taxonomy finding, and no
  migration step renames the id or the path

#### Scenario: A rename breaks the workspace

- **WHEN** any registry `id` is changed to match its directory name
- **THEN** the declaration identity check, the capability-adoption provider
  binding, the decisions-register reference check, and the promotion ledger
  for that id all fail, and the change is rejected rather than repaired by
  updating the reference sites
