# developer-workflow-facade Specification

## Purpose
Forge-owned modules sit in the same checkout as the CLI, but no command found
them by declaration or ran one module's checks. A `forge module list` and `forge
module verify` pair builds its catalog from local `.project.json` and Gate
declarations and delegates verification once to the existing `forge gate`
handler, keeping that handler's evidence and exit semantics. The facade stays a
local source-checkout concern and never triggers hosted CI, publication or
deployment.
## Requirements
### Requirement: Discover Forge-owned modules from local declarations

Forge SHALL provide `forge module list` that discovers only immediate child
directories under the executable-relative Forge checkout's `modules/` which
contain a valid `.project.json` identity and a supported Gate declaration
(`gate.toml` or `.ai-gate/gate.yaml`). It SHALL validate project IDs, required
stack/profile metadata, canonical path containment and uniqueness, then render
the catalog sorted by ID in human and JSON formats. Children without a
`.project.json` are ignored. A declared child with malformed metadata, no Gate
declaration, duplicate identity or an escaping symlink SHALL fail closed.

#### Scenario: List consolidated modules in stable order

- **WHEN** Forge contains valid declared modules under `modules/`
- **THEN** `forge module list` prints each module ID, stack, profile and
  repository-relative path sorted by ID, and JSON format returns the same
  records

#### Scenario: Ignore undeclared module folders

- **WHEN** an immediate child under `modules/` has no `.project.json`
- **THEN** it is omitted and is never treated as an executable workflow

#### Scenario: Refuse invalid declarations and path escapes

- **WHEN** a declared module has malformed JSON, invalid or duplicate ID,
  missing stack/profile, no Gate declaration or a canonical path outside
  `modules/`
- **THEN** listing and verification return a typed diagnostic naming the
  module/path and run no Gate check

#### Scenario: Missing local source modules are explicit

- **WHEN** an installed Forge binary cannot resolve a local checkout with a
  `modules/` directory
- **THEN** the command reports `module-catalog-unavailable` and explains that
  module verification requires a Forge source checkout, without scanning CWD
  or a sibling path

### Requirement: Verify one module through the existing Forge Gate

Forge SHALL provide `forge module verify <ID>` which resolves an exact ID from
the validated module catalog and delegates once to the existing `forge gate`
handler with the canonical module path. It SHALL pass `--dry-run`, Fast/Full
profile and timeout options through, retain existing Gate output, evidence,
redaction, persistence and exit semantics, and never invoke hosted CI,
publication, deployment, a shell or an arbitrary path.

#### Scenario: Verify a selected module

- **WHEN** a maintainer runs `forge module verify rust-platform-libs`
- **THEN** Forge executes only that module's declared Gate checks using Forge
  Gate and returns the same evidence and verdict exit behavior as
  `forge gate <module-path>`

#### Scenario: Rehearse without executing or persisting

- **WHEN** a maintainer runs `forge module verify <ID> --dry-run`
- **THEN** Forge returns the existing Gate rehearsal and creates no module
  Gate evidence or journal record

#### Scenario: Unknown ID does not become a filesystem path

- **WHEN** a maintainer runs `forge module verify <unknown-id>`
- **THEN** Forge returns `module-not-found`, does not execute a path derived
  from the input, and does not fall back to CWD, PATH or a sibling checkout

#### Scenario: Gate failure remains a Gate verdict

- **WHEN** a selected module's Gate checks fail, time out or are unavailable
- **THEN** the module command preserves the existing Gate evidence and
  blocking/exit semantics rather than reporting facade success

### Requirement: Keep the module facade separate from remote delivery

Forge SHALL describe this module interface as local source-checkout
verification. It SHALL NOT trigger GitHub Actions, package publication,
deployment, `forge use` binary routing or arbitrary browser command execution.

#### Scenario: Remote operations remain separate

- **WHEN** a maintainer lists or verifies a module
- **THEN** only local discovery and the existing local Gate runtime are used;
  no remote write or provider call occurs

#### Scenario: Existing facades remain compatible

- **WHEN** a user invokes `forge gate <PATH>`, `forge use <PRODUCT>`, or the
  web command catalog
- **THEN** their existing behavior remains unchanged and the new module
  commands are discoverable with accurate CLI-only disposition
