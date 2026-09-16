# project-import Specification

## Purpose
TBD - created by archiving change project-import. Update Purpose after archive.
## Requirements
### Requirement: Evidence-backed detection

Forge SHALL implement forge import with evidence-backed detection of the complete import inventory and a suggested profile and maturity that distinguishes unknown from missing.

#### Scenario: Evidence-backed detection success

- **WHEN** a recognizable existing repository is inspected
- **THEN** the proposal identifies detected files, stack, package manager, infrastructure and suggested profile

#### Scenario: Evidence-backed detection failure

- **WHEN** detectors disagree about the repository root or profile
- **THEN** import reports ambiguity and requires an explicit selection before writes

#### Scenario: Evidence-backed detection boundary

- **WHEN** the project has no Git remote or DriftWatch configuration
- **THEN** the proposal marks those facts as missing without preventing read-only inspection

### Requirement: Minimal and repeatable adoption

Forge SHALL create a validated manifest and register the project only after its import proposal is accepted; repeated import SHALL preserve identity and unrelated content.

#### Scenario: Minimal and repeatable adoption success

- **WHEN** the user accepts a valid import proposal
- **THEN** only the manifest and registry are changed and the project is inspectable

#### Scenario: Minimal and repeatable adoption failure

- **WHEN** the manifest destination is unwritable or existing metadata conflicts
- **THEN** import fails without changing source or replacing conflicting metadata

#### Scenario: Minimal and repeatable adoption boundary

- **WHEN** an unchanged project is imported again
- **THEN** the same registry identity is retained without duplicate records

