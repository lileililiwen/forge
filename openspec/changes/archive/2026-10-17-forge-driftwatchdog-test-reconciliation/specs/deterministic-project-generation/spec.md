## MODIFIED Requirements

### Requirement: Workspace-compatible project metadata

Generation SHALL write a sibling-schema-compatible `.project.json` carrying
the project id, a governance profile from an explicit per-profile mapping,
the profile's native verification command, `evidence_status: planned`, and
non-deployable defaults, and SHALL omit the file — never guess — when the
profile has no mapping or the operator opts out.

#### Scenario: Opt-out parity

- **WHEN** generation runs with `--no-workspace-metadata`
- **THEN** Forge omits `.project.json` and its ownership receipt, and all
  other generated project content remains byte-identical to an otherwise
  equivalent generation with metadata enabled, excluding only the actual
  `generated_at` value in the standard-pack receipt

#### Scenario: Mapped profile generated

- **WHEN** `forge new` creates a project from a profile with a governance
  mapping
- **THEN** the output tree contains `.project.json` whose verification
  command equals the profile's native test command and whose
  evidence_status is planned

#### Scenario: Unmapped profile

- **WHEN** the chosen profile declares no governance mapping
- **THEN** no metadata file is written, a note names the missing mapping,
  and every other generation output is unchanged
