# extended-profile-catalog Specification

## Purpose
Sixth supported profile react-web with a tested native-buildable template under the established profile contract, plus reserved specialist candidates discoverable as planned but refused before any file change.
## Requirements
### Requirement: React web generation support

Forge SHALL add a versioned react-web descriptor and deterministic native-buildable template using the established profile contract.

#### Scenario: React web generation support success

- **WHEN** a user selects a verified react-web version
- **THEN** new generates the declared layout and native build/test commands succeed

#### Scenario: React web generation support failure

- **WHEN** react-web is selected with an incompatible backend-only feature
- **THEN** resolution rejects the unsupported combination before generation

#### Scenario: React web generation support boundary

- **WHEN** the v0.1 catalog is inspected for historical compatibility
- **THEN** the original five profile IDs remain valid

### Requirement: Explicit catalog support status

Forge SHALL distinguish supported profiles from proposed specialist profiles and require compatibility, template and validation evidence before promotion.

#### Scenario: Explicit catalog support status success

- **WHEN** a specialist profile has all required evidence
- **THEN** its published version becomes selectable through the same resolver

#### Scenario: Explicit catalog support status failure

- **WHEN** a candidate lacks template or test evidence
- **THEN** generation refuses it with a planned or unsupported status

#### Scenario: Explicit catalog support status boundary

- **WHEN** a client profile needs server capabilities
- **THEN** the catalog describes a separate backend boundary rather than embedding server infrastructure in the client

