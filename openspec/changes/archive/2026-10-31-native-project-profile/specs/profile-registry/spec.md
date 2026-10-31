## ADDED Requirements

### Requirement: Native profiles are known and not generatable

Forge SHALL recognize a native profile as a known descriptor that a project may
register and be assessed against, while refusing to generate one. A native
profile SHALL be excluded from the selectable catalog and SHALL carry no
generated capability set.

#### Scenario: A native project registers

- **WHEN** a manifest names a native profile and requests no capabilities
- **THEN** `forge register` succeeds and the project is inspectable

#### Scenario: Generation is refused

- **WHEN** `forge new` is asked for a native profile
- **THEN** it refuses before staging any file or writing a registry row, naming
  the profile as native and having no scaffold

#### Scenario: A capability is requested

- **WHEN** a manifest requests a generated capability on a native profile
- **THEN** resolution refuses with `incompatible-profile`, because a native
  project has no generated feature set

#### Scenario: The selectable catalog is unchanged

- **WHEN** `list_profiles` is called
- **THEN** only supported profiles are listed; a native profile is reachable
  through inspection, not selection
