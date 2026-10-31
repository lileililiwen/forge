## ADDED Requirements

### Requirement: Doctor assesses a native project

Doctor SHALL assess a project whose manifest names a native profile as a known
project rather than as an unknown one. Its profile check SHALL pass, its build
definition SHALL be judged against the descriptor's stack, and its feature check
SHALL pass only when no capabilities are requested.

#### Scenario: Native profile is known

- **WHEN** doctor inspects a manifest with a native profile
- **THEN** `profile-known` is `PASS` and descriptor-gated checks run instead of
  being unavailable

#### Scenario: Native build definition

- **WHEN** the native descriptor declares a Cargo workspace
- **THEN** `build-config` is `PASS` when `Cargo.toml` is present and `FAIL` with
  that expectation when it is missing

#### Scenario: Native feature request

- **WHEN** the manifest requests a generated capability on a native profile
- **THEN** `features-compatible` is `FAIL`, because the native profile has no
  generated feature set

#### Scenario: No false planned gap

- **WHEN** the checker or API status projects readiness for a native profile
- **THEN** it does not report the profile as a planned candidate with no
  template
