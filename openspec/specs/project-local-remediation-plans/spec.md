# project-local-remediation-plans Specification

## Purpose
TBD - created by archiving change project-local-remediation-plans. Update Purpose after archive.
## Requirements
### Requirement: Versioned, previewable remediation plans

Forge SHALL convert supported automatic findings into a versioned remediation
plan, contract `forge-remediation-plan/0.1.0`, that names each action, its
target path, its ownership, its preconditions and the file digests it expects,
and SHALL let a caller preview it with `scan`, `plan` and `diff` without writing.

#### Scenario: A plan is previewed without writing

- **WHEN** a caller runs `scan`, `plan` or `diff`
- **THEN** the plan, the per-file diff and the outcome are rendered and no file,
  registry byte or provider value is written

#### Scenario: Only automatic findings become actions

- **WHEN** a finding is classified `semantic` or `manual`
- **THEN** it is refused as an action with a typed error naming its class, and no
  plan is emitted for it

#### Scenario: Finding belongs to another project

- **WHEN** a caller selects a `gaps.ci.<project>.ci` finding whose project id
  differs from the target manifest
- **THEN** Forge refuses with `remediation-invalid` before producing actions

#### Scenario: Scan identifies a missing local CI asset

- **WHEN** a caller runs `scan` against a valid target without
  `.standard/ci/verify.yml`
- **THEN** Forge reports the automatic `gaps.ci.<project>.ci` finding without
  requiring a pack selector or writing project or registry files

### Requirement: Ownership-safe, idempotent apply

Forge SHALL require explicit confirmation before applying, SHALL write only
paths Forge owns, SHALL refuse an unowned collision, and SHALL be idempotent
when re-applied with unchanged inputs.

#### Scenario: An unowned collision is refused

- **WHEN** a planned action targets a file Forge does not own
- **THEN** the apply is refused with a typed conflict naming the path and the
  existing bytes are preserved

#### Scenario: Re-applying an unchanged plan is a no-op

- **WHEN** a confirmed plan is applied twice against unchanged inputs
- **THEN** the second apply writes nothing and reports the actions as already
  applied

#### Scenario: A failed apply rolls back

- **WHEN** an apply fails part-way through promoting staged files
- **THEN** the prior bytes are restored and a failed/rolled-back outcome with
  rollback information is recorded

#### Scenario: A saved plan becomes stale

- **WHEN** an affected file changes after a plan is produced
- **THEN** applying the saved plan returns `remediation-conflict` and preserves
  the changed bytes

#### Scenario: Standard asset path escapes through a symlink

- **WHEN** a planned `.standard/` path resolves outside the target through a
  symlink
- **THEN** Forge refuses before writing outside the target

### Requirement: Bounded repair that never leaks or reaches out

Forge SHALL source CI and Compose assets only from a selected standard pack
version, SHALL NOT generate or disclose secrets, and SHALL NOT push, tag or
mutate any remote provider.

#### Scenario: An asset must come from a selected pack

- **WHEN** a plan installs a CI or Compose asset
- **THEN** the asset is resolved from a named standard-pack version, and a
  missing asset is a typed refusal rather than a guessed file

#### Scenario: No secret is created or printed

- **WHEN** a repair needs a credential
- **THEN** Forge references it without generating, storing or printing it
