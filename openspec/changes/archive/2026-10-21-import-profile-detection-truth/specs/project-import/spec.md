## MODIFIED Requirements

### Requirement: Evidence-backed detection

Forge SHALL implement forge import with evidence-backed detection of the complete import inventory and a suggested profile and maturity that distinguishes unknown from missing. The detection SHALL cover every profile the catalogue lists as supported, and an accepted manifest SHALL carry the detected language.

#### Scenario: Evidence-backed detection success

- **WHEN** a recognizable existing repository is inspected
- **THEN** the proposal identifies detected files, stack, package manager, infrastructure and suggested profile

#### Scenario: Evidence-backed detection failure

- **WHEN** detectors disagree about the repository root or profile
- **THEN** import reports ambiguity and requires an explicit selection before writes

#### Scenario: Evidence-backed detection boundary

- **WHEN** the project has no Git remote or DriftWatch configuration
- **THEN** the proposal marks those facts as missing without preventing read-only inspection

#### Scenario: Every supported profile is detected

- **WHEN** a repository carrying the markers of a supported profile is inspected
- **THEN** the proposal names that profile with its language and its package manager, and the manifest written on acceptance carries that language
